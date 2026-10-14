//! Strict serialized fixture request parsing and canonical hash verification.
use super::types::*;
use super::{boundary, dual_domain, source_coupling};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub(crate) const NORMALIZED_SCHEMA: &str = "solute-normalized-request-v1";
pub(crate) const LEGACY_SCHEMA: &str = "solute-request-v1";

/// Scan JSON syntax before Value parsing so duplicate object keys cannot be overwritten.
fn reject_duplicate_keys(text: &str) -> Result<(), String> {
    fn ws(b: &[u8], p: &mut usize) {
        while *p < b.len() && b[*p].is_ascii_whitespace() {
            *p += 1
        }
    }
    fn string(b: &[u8], p: &mut usize) -> Result<String, String> {
        let start = *p;
        *p += 1;
        let mut escaped = false;
        while *p < b.len() {
            let c = b[*p];
            *p += 1;
            if escaped {
                escaped = false
            } else if c == b'\\' {
                escaped = true
            } else if c == b'"' {
                return serde_json::from_slice(&b[start..*p]).map_err(|e| e.to_string());
            }
        }
        Err("unterminated JSON string".into())
    }
    fn value(b: &[u8], p: &mut usize) -> Result<(), String> {
        ws(b, p);
        match b.get(*p) {
            Some(b'{') => {
                *p += 1;
                ws(b, p);
                let mut keys = HashSet::new();
                if b.get(*p) == Some(&b'}') {
                    *p += 1;
                    return Ok(());
                }
                loop {
                    ws(b, p);
                    if b.get(*p) != Some(&b'"') {
                        return Err("invalid JSON object key".into());
                    }
                    let key = string(b, p)?;
                    if !keys.insert(key.clone()) {
                        return Err(format!("duplicate JSON key: {key}"));
                    }
                    ws(b, p);
                    if b.get(*p) != Some(&b':') {
                        return Err("expected colon".into());
                    }
                    *p += 1;
                    value(b, p)?;
                    ws(b, p);
                    match b.get(*p) {
                        Some(b',') => *p += 1,
                        Some(b'}') => {
                            *p += 1;
                            break;
                        }
                        _ => return Err("invalid JSON object delimiter".into()),
                    }
                }
                Ok(())
            }
            Some(b'[') => {
                *p += 1;
                ws(b, p);
                if b.get(*p) == Some(&b']') {
                    *p += 1;
                    return Ok(());
                }
                loop {
                    value(b, p)?;
                    ws(b, p);
                    match b.get(*p) {
                        Some(b',') => *p += 1,
                        Some(b']') => {
                            *p += 1;
                            break;
                        }
                        _ => return Err("invalid JSON array delimiter".into()),
                    }
                }
                Ok(())
            }
            Some(b'"') => string(b, p).map(|_| ()),
            Some(_) => {
                let start = *p;
                while *p < b.len()
                    && !matches!(b[*p], b',' | b']' | b'}')
                    && !b[*p].is_ascii_whitespace()
                {
                    *p += 1
                }
                if *p == start {
                    Err("invalid JSON value".into())
                } else {
                    Ok(())
                }
            }
            None => Err("missing JSON value".into()),
        }
    }
    let b = text.as_bytes();
    let mut p = 0;
    value(b, &mut p)?;
    ws(b, &mut p);
    if p != b.len() {
        return Err("trailing JSON data".into());
    }
    Ok(())
}

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
    if let Some(v) = &fields.reactions.ordered_species {
        if let Explicit::Available(species) = &v.value {
            let mut seen = vec![false; species.len()];
            for s in species {
                if s.position == 0 || s.position > species.len() || seen[s.position - 1] {
                    return Err(
                        "ordered_species positions must be a one-based contiguous order".into(),
                    );
                }
                seen[s.position - 1] = true;
            }
            if let Some(conc) = &fields.reactions.initial_dissolved_concentration {
                if let Explicit::Available(conc) = &conc.value {
                    if conc.len() != species.len() {
                        return Err(
                            "initial_dissolved_concentration length must match ordered_species"
                                .into(),
                        );
                    }
                }
            }
            if let Some(count) = species_count_value(fields) {
                if count != species.len() {
                    return Err("species_count must match ordered_species length".into());
                }
            }
        }
    }
    let nodes = fields
        .transport_properties
        .coordinates
        .as_ref()
        .and_then(|v| match &v.value {
            Explicit::Available(c) => Some(c.nodes),
            _ => None,
        });
    for (name, len) in [
        (
            "temperature_history",
            available_len(&fields.transport_properties.temperature_history),
        ),
        (
            "initial_pressure_head_profile",
            available_len(&fields.transport_properties.initial_pressure_head_profile),
        ),
        (
            "initial_temperature_profile",
            available_len(&fields.transport_properties.initial_temperature_profile),
        ),
        (
            "initial_water_content",
            available_len(&fields.nonequilibrium.initial_water_content),
        ),
        (
            "initial_dual_immobile_water_content",
            available_len(&fields.nonequilibrium.initial_dual_immobile_water_content),
        ),
        (
            "dynamic_exchange_sink",
            available_len(&fields.nonequilibrium.dynamic_exchange_sink),
        ),
    ] {
        if let (Some(nodes), Some(len)) = (nodes, len) {
            if len != nodes {
                return Err(format!(
                    "{name} length {len} does not match coordinates nodes {nodes}"
                ));
            }
        }
    }
    if let (Some(temp), Some(pressure)) = (
        available_len(&fields.transport_properties.initial_temperature_profile),
        available_len(&fields.transport_properties.initial_pressure_head_profile),
    ) {
        if temp != pressure {
            return Err(
                "initial_temperature_profile and initial_pressure_head_profile lengths differ"
                    .into(),
            );
        }
    }
    if let Some(dual) = &fields.nonequilibrium.dual_domain {
        if let Explicit::Available(dual) = &dual.value {
            if dual.dynamic_exchange_claimed && dual.i_dual_por == 0 {
                return Err("dynamic exchange requires iDualPor > 0".into());
            }
            if dual.i_dual_por == 0 {
                if dual.water_model != 0 {
                    return Err("single-domain control must use water_model 0".into());
                }
            } else if !matches!(dual.water_model, 6 | 7) {
                return Err("dual-domain water model must be 6 or 7".into());
            }
        }
    }
    if let Some(exchange) = &fields.nonequilibrium.dual_porosity_exchange_state {
        if let Explicit::Available(state) = &exchange.value {
            if let Some(dual) = &fields.nonequilibrium.dual_domain {
                if let Explicit::Available(dual) = &dual.value {
                    // One source variable, two spellings: the `iDualPor` of the
                    // guard at `43` is also the bundle's `dual_porosity`.
                    if dual.i_dual_por != state.dual_porosity {
                        return Err(
                            "dual_porosity_exchange_state.dual_porosity must match dual_domain.iDualPor"
                                .into(),
                        );
                    }
                    // The reader remaps models 6 and 7 to `iModel=0` before it
                    // reads ParD (INPUT.FOR:619-624), and only a model 7 deck
                    // fills ParD(10,M) and ParD(11,M) (INPUT.FOR:623, NPar=11).
                    if matches!(dual.water_model, 6 | 7) && state.hydraulic_model != 0 {
                        return Err(
                            "dual-domain water models 6 and 7 reach DualPor as iModel 0".into()
                        );
                    }
                    if dual.water_model == 7 {
                        for (index, material) in state.materials.iter().enumerate() {
                            if material.immobile_n.is_none()
                                || material.immobile_saturated_conductivity.is_none()
                                || material.mobile_tortuosity.is_none()
                            {
                                return Err(format!(
                                    "water model 7 fills ParD(6,M), ParD(10,M) and ParD(11,M), so material {index} must carry them"
                                ));
                            }
                        }
                    }
                }
            }
            // The kernel owns the shape and range checks. It runs after the
            // cross-field checks above, which name the request-level cause first.
            dual_domain::validate(state)
                .map_err(|e| format!("dual_porosity_exchange_state: {e}"))?;
        }
    }
    if let Some(chpar) = &fields.reactions.material_species_chpar {
        if let Explicit::Available(records) = &chpar.value {
            let mut keys = HashSet::new();
            for record in records {
                if record.species == 0
                    || record.material == 0
                    || !keys.insert((record.species, record.material))
                {
                    return Err(
                        "material_species_chpar requires unique positive species/material indices"
                            .into(),
                    );
                }
            }
        }
    }
    if let Some(state) = &fields.transport.endpoint_state {
        if let Explicit::Available(s) = &state.value {
            // Shape, range and finiteness live in the kernel. The codes are only
            // read when both are available; `validate_required_mode_inputs` rejects
            // a missing code, so the placeholder 0 only keeps the shape checks on.
            let code = |c: &Option<UnitValue<i32>>| match c.as_ref().map(|c| &c.value) {
                Some(Explicit::Available(code)) => *code,
                _ => 0,
            };
            boundary::validate(
                s,
                code(&fields.transport.lower_boundary_code),
                code(&fields.transport.upper_boundary_code),
            )
            .map_err(|e| format!("endpoint_primitives: {e}"))?;
            // TDep is one-dimensional with a 16-entry stride per species
            // (INPUT.FOR:1379-1390); the atmospheric code reads slots 6, 9, 10.
            let species_base = (s.source_species - 1) * 16;
            if s.source_tdep
                .as_ref()
                .is_some_and(|v| v.len() < species_base + 10)
            {
                return Err("endpoint_primitives.source_tdep must cover TDep((jS-1)*16+10)".into());
            }
            if s.source_chpar.iter().flatten().any(|record| {
                record.species == 0
                    || record.material == 0
                    || record.material > s.mobile_immobile.len()
            }) {
                return Err(
                    "endpoint_primitives.source_chpar must name a known species and material"
                        .into(),
                );
            }
            if let Some(code) = fields.transport.upper_boundary_code.as_ref() {
                if matches!(&code.value, Explicit::Available(-2)) {
                    let complete = s.source_chpar.is_some()
                        && s.source_tdep.is_some()
                        && s.old_temperature.is_some()
                        && s.new_temperature.is_some()
                        && s.top_surface_distance.is_some()
                        && s.top_atmospheric_concentration.is_some();
                    if !complete {
                        return Err("atmospheric endpoint requires ChPar/TDep, both temperatures, surface distance, and atmospheric concentration".into());
                    }
                    // MatNum(N) (884, 917, 207) is the only material the endpoint
                    // atmosphere reads.
                    let top_material = s.material_ids.last().copied().unwrap_or(0);
                    if !s.source_chpar.as_ref().is_some_and(|records| {
                        records
                            .iter()
                            .any(|r| r.species == s.source_species && r.material == top_material)
                    }) {
                        return Err("atmospheric endpoint requires a source ChPar record for the species at the top node material".into());
                    }
                }
            }
        }
    }
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
    // Source-coupling guard cross-checks. Each bundle is only reachable through one
    // call, and each call hangs from one guard, so a bundle whose guard cannot fire is
    // a request that believes a flag value the control record does not carry.
    let control = fields.source_coupling.control_switches.as_ref();
    if let Some(UnitValue {
        value: Explicit::Available(state),
        ..
    }) = &fields.source_coupling.root_uptake_state
    {
        if control.is_none() {
            return Err("root_uptake_state requires the control-section SinkF mapping".into());
        }
        if !switches.root_sink {
            return Err("root_uptake_state requires SinkF, the SOLUTE.FOR:94 call guard".into());
        }
        // `INPUT.FOR:1122` forces `lActRSU=.false.` as soon as `NS.gt.1`, and the
        // `OmegaS,SPot,rKM,cMin,lOmegaW` read of `INPUT.FOR:1126` is gated on
        // `lActRSU.and.NS.eq.1`, so active uptake with several species has no reader.
        if state.active_uptake && state.species_count != 1 {
            return Err(
                "lActRSU requires species_count 1, INPUT.FOR:1122 disables it above that".into(),
            );
        }
        source_coupling::validate_uptake(state).map_err(|e| format!("root_uptake_state: {e}"))?;
    }
    if let Some(UnitValue {
        value: Explicit::Available(state),
        ..
    }) = &fields.source_coupling.size_exclusion_state
    {
        if control.is_none() {
            return Err("size_exclusion_state requires the control-section lWat mapping".into());
        }
        // `HYDRUS.FOR:390` is `if(lBact.and..not.lWat)`: both terms are guard terms,
        // and `lBact` is not a free flag but `iBact.eq.1` (`INPUT.FOR:1313-1314`),
        // which `derive_switches` already reads out of `biology_mode`.
        if !switches.bacteria {
            return Err(
                "size_exclusion_state requires iBact=1, HYDRUS.FOR:390 guards on lBact".into(),
            );
        }
        if switches.water_flow {
            return Err(
                "size_exclusion_state requires lWat=false, HYDRUS.FOR:390 guards on \
                 .not.lWat"
                    .into(),
            );
        }
        source_coupling::validate_exclusion(state)
            .map_err(|e| format!("size_exclusion_state: {e}"))?;
    }
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
                3 => BlockingMode::SourceNoOp3,
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

fn species_count_value(fields: &RequestFields) -> Option<usize> {
    fields
        .transport_properties
        .species_count
        .as_ref()
        .and_then(|v| match &v.value {
            Explicit::Available(n) => Some(*n),
            _ => None,
        })
}

fn available_len(v: &Option<UnitValue<Vec<DefaultReal>>>) -> Option<usize> {
    v.as_ref().and_then(|v| match &v.value {
        Explicit::Available(x) => Some(x.len()),
        _ => None,
    })
}
fn explicit_from_option_vec(
    v: &Option<UnitValue<Vec<DefaultReal>>>,
    availability: &str,
    reason: &str,
) -> Explicit<Vec<DefaultReal>> {
    v.as_ref()
        .map(|v| v.value.clone())
        .unwrap_or_else(|| unavailable(availability, reason))
}
fn unavailable<T>(availability: &str, reason: &str) -> Explicit<T> {
    Explicit::Unavailable(UnavailableValue {
        availability: availability.into(),
        reason: reason.into(),
    })
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
fn finite_tree(v: &Value) -> bool {
    match v {
        Value::Number(n) => n.as_f64().is_some_and(f64::is_finite),
        Value::Array(a) => a.iter().all(finite_tree),
        Value::Object(o) => o.values().all(finite_tree),
        _ => true,
    }
}
fn validate_field(name: &str, v: &Value) -> Result<(), String> {
    let o = v
        .as_object()
        .ok_or_else(|| format!("field {name}: expected object"))?;
    for key in o.keys() {
        if !matches!(
            key.as_str(),
            "units" | "value" | "availability" | "reason" | "provenance"
        ) {
            return Err(format!("field {name}: unknown key {key}"));
        }
    }
    let _ = unit(v)?;
    let has_value = v.get("value").is_some();
    let has_unavailable = v.get("availability").is_some() || v.get("reason").is_some();
    if has_value && has_unavailable {
        return Err(format!(
            "field {name}: value and unavailable metadata are mutually exclusive"
        ));
    }
    if let Some(value) = v.get("value") {
        if !finite_tree(value) {
            return Err(format!("field {name}: nonfinite value"));
        }
    } else if v
        .get("availability")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
        || v.get("reason")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        return Err(format!(
            "field {name}: unavailable field requires availability and reason"
        ));
    }
    Ok(())
}
fn unit(v: &Value) -> Result<&str, String> {
    v.get("units")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "explicit units required".to_string())
}
fn expect_units(v: &Value, expected: &str, name: &str) -> Result<(), String> {
    let got = unit(v)?;
    if got != expected {
        Err(format!(
            "field {name}: expected units {expected}, got {got}"
        ))
    } else {
        Ok(())
    }
}
fn field_value<'a>(
    fields: &'a Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<&'a Value>, String> {
    let Some(v) = fields.get(name) else {
        return Ok(None);
    };
    expect_units(v, units, name)?;
    Ok(Some(v))
}
fn explicit<T>(
    v: &Value,
    f: impl FnOnce(&Value) -> Result<T, String>,
) -> Result<Explicit<T>, String> {
    if let Some(value) = v.get("value") {
        Ok(Explicit::Available(f(value)?))
    } else {
        Ok(Explicit::Unavailable(UnavailableValue {
            availability: v
                .get("availability")
                .and_then(Value::as_str)
                .unwrap()
                .into(),
            reason: v.get("reason").and_then(Value::as_str).unwrap().into(),
        }))
    }
}
fn as_f64(v: &Value, name: &str) -> Result<f64, String> {
    v.as_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| format!("field {name}: expected finite number"))
}
fn as_u8(v: &Value, name: &str) -> Result<u8, String> {
    let n = v
        .as_u64()
        .ok_or_else(|| format!("field {name}: expected nonnegative integer"))?;
    u8::try_from(n).map_err(|_| format!("field {name}: integer out of range"))
}
fn as_i32(v: &Value, name: &str) -> Result<i32, String> {
    let n = v
        .as_i64()
        .ok_or_else(|| format!("field {name}: expected integer"))?;
    i32::try_from(n).map_err(|_| format!("field {name}: integer out of range"))
}
fn as_usize(v: &Value, name: &str) -> Result<usize, String> {
    let n = v
        .as_u64()
        .ok_or_else(|| format!("field {name}: expected nonnegative integer"))?;
    usize::try_from(n).map_err(|_| format!("field {name}: integer out of range"))
}
fn as_bool(v: &Value, name: &str) -> Result<bool, String> {
    v.as_bool()
        .ok_or_else(|| format!("field {name}: expected boolean"))
}
fn as_string(v: &Value, name: &str) -> Result<String, String> {
    v.as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("field {name}: expected nonempty string"))
}
fn number_vec(value: &Value, name: &str) -> Result<Vec<DefaultReal>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("field {name}: expected array"))?
        .iter()
        .map(|x| Ok(DefaultReal::from_input(as_f64(x, name)?)))
        .collect()
}

fn unit_value<T>(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
    parser: impl FnOnce(&Value) -> Result<T, String>,
) -> Result<Option<UnitValue<T>>, String> {
    let Some(v) = field_value(fields, name, units)? else {
        return Ok(None);
    };
    Ok(Some(UnitValue {
        units: units.into(),
        value: explicit(v, parser)?,
    }))
}
fn maybe_number(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<DefaultReal>>, String> {
    unit_value(fields, name, units, |x| {
        Ok(DefaultReal::from_input(as_f64(x, name)?))
    })
}
fn maybe_bool(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<bool>>, String> {
    unit_value(fields, name, units, |x| as_bool(x, name))
}
fn maybe_i32(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<i32>>, String> {
    unit_value(fields, name, units, |x| as_i32(x, name))
}
fn maybe_usize(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<usize>>, String> {
    unit_value(fields, name, units, |x| as_usize(x, name))
}
fn maybe_string(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<String>>, String> {
    unit_value(fields, name, units, |x| as_string(x, name))
}
fn maybe_number_vec(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<Vec<DefaultReal>>>, String> {
    unit_value(fields, name, units, |x| number_vec(x, name))
}

/// `ChPar((jS-1)*16+slot, M)` records: the source reads four material slots and
/// 16 slots per species, indexing the material in the second array dimension
/// (`INPUT.FOR:1333-1352`).
fn chpar_records(value: &Value, name: &str) -> Result<Vec<MaterialSpeciesParameters>, String> {
    let entries = value
        .as_array()
        .ok_or_else(|| format!("{name} value must be an array"))?;
    entries
        .iter()
        .map(|entry| {
            let o = entry
                .as_object()
                .ok_or_else(|| format!("{name} entries must be objects"))?;
            validate_object_keys(name, o, &["species", "material", "slots"])?;
            let species = o
                .get("species")
                .map(|v| as_usize(v, &format!("{name}.species")))
                .transpose()?
                .ok_or_else(|| format!("{name}.species missing"))?;
            let material = o
                .get("material")
                .map(|v| as_usize(v, &format!("{name}.material")))
                .transpose()?
                .ok_or_else(|| format!("{name}.material missing"))?;
            let slots = number_vec(
                o.get("slots")
                    .ok_or_else(|| format!("{name}.slots missing"))?,
                &format!("{name}.slots"),
            )?;
            if slots.len() != 20 {
                return Err(format!("{name} slots must have length 20"));
            }
            Ok(MaterialSpeciesParameters {
                species,
                material,
                slots,
            })
        })
        .collect()
}

fn maybe_chpar(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<Vec<MaterialSpeciesParameters>>>, String> {
    unit_value(
        fields,
        "material_species_chpar",
        "source ChPar slots by species/material",
        |x| chpar_records(x, "material_species_chpar"),
    )
}

fn maybe_endpoint_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<EndpointPrimitiveState>>, String> {
    unit_value(
        fields,
        "endpoint_primitives",
        "source primitive endpoint state",
        |x| {
            let o = x
                .as_object()
                .ok_or("endpoint_primitives value must be an object")?;
            let array = |name: &str| -> Result<Vec<DefaultReal>, String> {
                number_vec(
                    o.get(name)
                        .ok_or_else(|| format!("endpoint_primitives.{name} missing"))?,
                    name,
                )
            };
            let scalar = |name: &str| -> Result<DefaultReal, String> {
                Ok(DefaultReal::from_input(as_f64(
                    o.get(name)
                        .ok_or_else(|| format!("endpoint_primitives.{name} missing"))?,
                    name,
                )?))
            };
            validate_object_keys(
                "endpoint_primitives",
                o,
                &[
                    "coordinates",
                    "old_water",
                    "new_water",
                    "old_velocity",
                    "new_velocity",
                    "old_dispersion",
                    "new_dispersion",
                    "old_retardation",
                    "new_retardation",
                    "old_concentration",
                    "material_immobile_water",
                    "source_g0",
                    "source_g1",
                    "upwind_weight",
                    "epsi",
                    "dt",
                    "alf",
                    "top_atmospheric_concentration",
                    "top_surface_distance",
                    "old_temperature",
                    "new_temperature",
                    "source_chpar",
                    "source_tdep",
                    "material_ids",
                    "old_matrix_velocity",
                    "new_matrix_velocity",
                    "mobile_immobile",
                    "source_species",
                    "bacteria",
                    "dual_porosity",
                    "bottom_resistance",
                    "bottom_prescribed",
                    "top_prescribed",
                    "new_accepted_concentration",
                    "t_level",
                    "vapor",
                    "r_min",
                ],
            )?;
            let opt_real = |name: &str| -> Result<Option<DefaultReal>, String> {
                o.get(name)
                    .map(|v| as_f64(v, name).map(DefaultReal::from_input))
                    .transpose()
            };
            let source_chpar = o
                .get("source_chpar")
                .map(|v| chpar_records(v, "endpoint_primitives.source_chpar"))
                .transpose()?;
            let opt_vec = |name: &str| -> Result<Option<Vec<DefaultReal>>, String> {
                o.get(name).map(|v| number_vec(v, name)).transpose()
            };
            let usize_vec = |name: &str| -> Result<Vec<usize>, String> {
                o.get(name)
                    .ok_or_else(|| format!("endpoint_primitives.{name} missing"))?
                    .as_array()
                    .ok_or_else(|| format!("endpoint_primitives.{name} must be an array"))?
                    .iter()
                    .map(|v| as_usize(v, name))
                    .collect()
            };
            let bool_vec = |name: &str| -> Result<Vec<bool>, String> {
                o.get(name)
                    .ok_or_else(|| format!("endpoint_primitives.{name} missing"))?
                    .as_array()
                    .ok_or_else(|| format!("endpoint_primitives.{name} must be an array"))?
                    .iter()
                    .map(|v| as_bool(v, name))
                    .collect()
            };
            Ok(EndpointPrimitiveState {
                coordinates: array("coordinates")?,
                old_water: array("old_water")?,
                new_water: array("new_water")?,
                old_velocity: array("old_velocity")?,
                new_velocity: array("new_velocity")?,
                old_dispersion: array("old_dispersion")?,
                new_dispersion: array("new_dispersion")?,
                old_retardation: array("old_retardation")?,
                new_retardation: array("new_retardation")?,
                old_concentration: array("old_concentration")?,
                material_immobile_water: array("material_immobile_water")?,
                source_g0: array("source_g0")?,
                source_g1: array("source_g1")?,
                upwind_weight: array("upwind_weight")?,
                epsi: scalar("epsi")?,
                dt: scalar("dt")?,
                alf: scalar("alf")?,
                top_atmospheric_concentration: opt_real("top_atmospheric_concentration")?,
                top_surface_distance: opt_real("top_surface_distance")?,
                old_temperature: opt_real("old_temperature")?,
                new_temperature: opt_real("new_temperature")?,
                source_chpar,
                source_tdep: opt_vec("source_tdep")?,
                material_ids: usize_vec("material_ids")?,
                mobile_immobile: bool_vec("mobile_immobile")?,
                source_species: o
                    .get("source_species")
                    .map(|v| as_usize(v, "source_species"))
                    .transpose()?
                    .unwrap_or(1),
                bacteria: o
                    .get("bacteria")
                    .map(|v| as_bool(v, "bacteria"))
                    .transpose()?
                    .unwrap_or(false),
                dual_porosity: o
                    .get("dual_porosity")
                    .map(|v| as_u8(v, "dual_porosity"))
                    .transpose()?
                    .unwrap_or(0),
                bottom_resistance: scalar("bottom_resistance")?,
                bottom_prescribed: scalar("bottom_prescribed")?,
                top_prescribed: scalar("top_prescribed")?,
                new_accepted_concentration: array("new_accepted_concentration")?,
                old_matrix_velocity: array("old_matrix_velocity")?,
                new_matrix_velocity: array("new_matrix_velocity")?,
                t_level: o
                    .get("t_level")
                    .map(|v| as_i32(v, "t_level"))
                    .transpose()?
                    .unwrap_or(0),
                vapor: o
                    .get("vapor")
                    .map(|v| as_bool(v, "endpoint_primitives.vapor"))
                    .transpose()?
                    .unwrap_or(false),
                r_min: scalar("r_min")?,
            })
        },
    )
}

fn maybe_moisture_dependency(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<MoistureDependencyRequest>>, String> {
    unit_value(
        fields,
        "moisture_dependency",
        "source selector and WDep table",
        |x| {
            let o = x
                .as_object()
                .ok_or("moisture_dependency value must be an object")?;
            validate_object_keys("moisture_dependency", o, &["selector", "table"])?;
            let selector = o
                .get("selector")
                .map(|v| as_u8(v, "moisture_dependency.selector"))
                .transpose()?
                .ok_or("moisture_dependency.selector missing")?;
            if selector > 2 {
                return Err("moisture_dependency.selector must be 0, 1, or 2".into());
            }
            let table = o
                .get("table")
                .map(|v| number_vec(v, "moisture_dependency.table"))
                .transpose()?
                .unwrap_or_default();
            if selector == 1 && table.is_empty() {
                return Err("selector 1 requires a WDep table".into());
            }
            Ok(MoistureDependencyRequest { selector, table })
        },
    )
}

fn maybe_orientation(
    fields: &Map<String, Value>,
    name: &str,
) -> Result<Option<UnitValue<FlowOrientation>>, String> {
    unit_value(fields, name, "orientation", |x| {
        match as_string(x, name)?.as_str() {
            "positive-native" => Ok(FlowOrientation::PositiveNative),
            "negative-native" => Ok(FlowOrientation::NegativeNative),
            _ => Err("boundary.flow_orientation must be positive-native or negative-native".into()),
        }
    })
}
fn maybe_coordinates(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<CoordinatesSpec>>, String> {
    unit_value(fields, "coordinates", "cm", |x| {
        let o = x.as_object().ok_or("coordinates value must be an object")?;
        validate_object_keys("coordinates", o, &["native", "canonical", "nodes"])?;
        let native = o
            .get("native")
            .map(|v| as_string(v, "coordinates.native"))
            .transpose()?
            .ok_or("coordinates.native missing")?;
        let canonical = o
            .get("canonical")
            .map(|v| as_string(v, "coordinates.canonical"))
            .transpose()?
            .ok_or("coordinates.canonical missing")?;
        let nodes = o
            .get("nodes")
            .map(|v| as_usize(v, "coordinates.nodes"))
            .transpose()?
            .ok_or("coordinates.nodes missing")?;
        if nodes == 0 {
            return Err("coordinates.nodes must be positive".into());
        }
        Ok(CoordinatesSpec {
            native,
            canonical,
            nodes,
        })
    })
}
fn maybe_transport_switches(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<SourceSwitches>>, String> {
    unit_value(fields, "transport_switches", "source flags", |x| {
        let obj = x
            .as_object()
            .ok_or("transport_switches value must be an object")?;
        validate_object_keys(
            "transport_switches",
            obj,
            &[
                "lArtD",
                "lTort",
                "iTort",
                "lTDep",
                "lVapor",
                "gas_terms",
                "lUpW",
            ],
        )?;
        let flag = |n: &str| -> Result<bool, String> {
            Ok(obj
                .get(n)
                .map(|v| as_bool(v, n))
                .transpose()?
                .unwrap_or(false))
        };
        Ok(SourceSwitches {
            upstream: flag("lUpW")?,
            artificial_dispersion: flag("lArtD")?,
            tortuosity: flag("lTort")?,
            temperature: flag("lTDep")?,
            vapor: flag("lVapor")?,
            gas_terms: flag("gas_terms")?,
            ..SourceSwitches::default()
        })
    })
}
fn maybe_species(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<Vec<SpeciesRecord>>>, String> {
    unit_value(fields, "ordered_species", "ordered species records", |x| {
        x.as_array()
            .ok_or("ordered_species value must be an array")?
            .iter()
            .map(|entry| {
                let o = entry
                    .as_object()
                    .ok_or("ordered_species entries must be objects")?;
                validate_object_keys(
                    "ordered_species",
                    o,
                    &[
                        "name",
                        "position",
                        "chain_rate_per_hour",
                        "initial_concentration_C_ref",
                        "linear_equilibrium_kd_cm3_per_g",
                    ],
                )?;
                Ok(SpeciesRecord {
                    name: o
                        .get("name")
                        .map(|v| as_string(v, "ordered_species.name"))
                        .transpose()?
                        .ok_or("ordered_species.name missing")?,
                    position: o
                        .get("position")
                        .map(|v| as_usize(v, "ordered_species.position"))
                        .transpose()?
                        .ok_or("ordered_species.position missing")?,
                    chain_rate_per_hour: DefaultReal::from_input(as_f64(
                        o.get("chain_rate_per_hour")
                            .ok_or("ordered_species.chain_rate_per_hour missing")?,
                        "ordered_species.chain_rate_per_hour",
                    )?),
                    initial_concentration: DefaultReal::from_input(as_f64(
                        o.get("initial_concentration_C_ref")
                            .ok_or("ordered_species.initial_concentration_C_ref missing")?,
                        "ordered_species.initial_concentration_C_ref",
                    )?),
                    linear_equilibrium_kd: DefaultReal::from_input(as_f64(
                        o.get("linear_equilibrium_kd_cm3_per_g")
                            .ok_or("ordered_species.linear_equilibrium_kd_cm3_per_g missing")?,
                        "ordered_species.linear_equilibrium_kd_cm3_per_g",
                    )?),
                })
            })
            .collect()
    })
}
fn maybe_dual_domain(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<DualDomainRequest>>, String> {
    unit_value(fields, "dual_domain", "source iDualPor selector", |x| {
        let o = x.as_object().ok_or("dual_domain value must be an object")?;
        validate_object_keys(
            "dual_domain",
            o,
            &["dynamic_exchange_claimed", "iDualPor", "water_model"],
        )?;
        Ok(DualDomainRequest {
            dynamic_exchange_claimed: o
                .get("dynamic_exchange_claimed")
                .map(|v| as_bool(v, "dual_domain.dynamic_exchange_claimed"))
                .transpose()?
                .unwrap_or(false),
            i_dual_por: o
                .get("iDualPor")
                .map(|v| as_u8(v, "dual_domain.iDualPor"))
                .transpose()?
                .ok_or("dual_domain.iDualPor missing")?,
            water_model: o
                .get("water_model")
                .map(|v| as_u8(v, "dual_domain.water_model"))
                .transpose()?
                .ok_or("dual_domain.water_model missing")?,
        })
    })
}

/// One `ParD(·,M)` column as the exchange reads it. `mobile_residual_water` and
/// `mobile_saturated_water` are `ParD(1,M)` and `ParD(2,M)`, i.e. the pair
/// `GenMat` turns into `thr(M)`/`ths(M)` (`INPUT.FOR:796-797`); there is no
/// `thr`/`ths` key, because a second spelling of that pair could contradict the
/// column. The three optional slots are the ones a model 6 deck leaves out
/// (`INPUT.FOR:618`, `NPar=9`); `validate_cross_field_dimensions` demands them
/// for a model 7 deck.
fn dual_porosity_material(record: &Value) -> Result<DualPorMaterial, String> {
    let name = "dual_porosity_exchange_state.materials";
    let o = record
        .as_object()
        .ok_or(format!("{name} records must be objects"))?;
    validate_object_keys(
        name,
        o,
        &[
            "mobile_residual_water",
            "mobile_saturated_water",
            "mobile_tortuosity",
            "immobile_residual_water",
            "immobile_saturated_water",
            "immobile_alpha",
            "immobile_n",
            "immobile_saturated_conductivity",
        ],
    )?;
    let required = |slot: &str| -> Result<DefaultReal, String> {
        Ok(DefaultReal::from_input(as_f64(
            o.get(slot)
                .ok_or_else(|| format!("{name}.{slot} missing"))?,
            slot,
        )?))
    };
    let optional = |slot: &str| -> Result<Option<DefaultReal>, String> {
        o.get(slot)
            .map(|v| as_f64(v, slot).map(DefaultReal::from_input))
            .transpose()
    };
    Ok(DualPorMaterial {
        mobile_residual_water: required("mobile_residual_water")?,
        mobile_saturated_water: required("mobile_saturated_water")?,
        mobile_tortuosity: optional("mobile_tortuosity")?,
        immobile_residual_water: required("immobile_residual_water")?,
        immobile_saturated_water: required("immobile_saturated_water")?,
        immobile_alpha: required("immobile_alpha")?,
        immobile_n: optional("immobile_n")?,
        immobile_saturated_conductivity: optional("immobile_saturated_conductivity")?,
    })
}

/// The primitive state of the active `DualPor` call (`WATFLOW.FOR:43-46`,
/// arguments `1027-1029`). Nothing in it is a routine output: `SinkIm` and
/// `WTransf` are results, and `ThNewIm` is written at `1068` and only reaches
/// `ThOldIm` through `Update` (`1112`), so the bundle carries `old_immobile_water`
/// as `ThOldIm` is read. `critical_atmospheric_head` is required and never
/// defaulted: `INPUT.FOR:67` comment-swallowes `hCritA` out of the data read and
/// `INPUT.FOR:102-103` assigns it `-abs(1.e+10)`. The patched reader the pinned
/// cases compile agrees, because the staged build copies `src_reference` verbatim
/// and then applies the pinned patch
/// (`tools/run_solute_verification_reference.py:393-398`), which rewrites only
/// `HYDRUS.FOR` and `TIME.FOR` (`reference/fortran_reference/PATCH_NOTES.md`,
/// "Scope"); the generated `gnu_compatibility.FOR`
/// (`tools/run_solute_verification_reference.py:403-405`) is a `GETDAT`/`GETTIM`
/// shim. No `patch/INPUT.f90` - and no `.f90` source of any kind - exists in this
/// tree, so `src_reference/INPUT.FOR` is the case reader on both the staged and the
/// patched path, and `hCritA` has no data-file source in either.
fn maybe_dual_porosity_exchange_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<DualPorPrimitiveState>>, String> {
    let name = "dual_porosity_exchange_state";
    unit_value(
        fields,
        name,
        "source primitive dual-porosity exchange state",
        |x| {
            let o = x
                .as_object()
                .ok_or(format!("{name} value must be an object"))?;
            validate_object_keys(
                name,
                o,
                &[
                    "coordinates",
                    "old_water",
                    "new_pressure_head",
                    "old_immobile_water",
                    "material_ids",
                    "materials",
                    "critical_atmospheric_head",
                    "dt",
                    "hydraulic_model",
                    "dual_porosity",
                ],
            )?;
            let array = |field: &str| -> Result<Vec<DefaultReal>, String> {
                number_vec(
                    o.get(field)
                        .ok_or_else(|| format!("{name}.{field} missing"))?,
                    field,
                )
            };
            let scalar = |field: &str| -> Result<DefaultReal, String> {
                Ok(DefaultReal::from_input(as_f64(
                    o.get(field)
                        .ok_or_else(|| format!("{name}.{field} missing"))?,
                    field,
                )?))
            };
            let materials = o
                .get("materials")
                .ok_or(format!("{name}.materials missing"))?
                .as_array()
                .ok_or(format!("{name}.materials must be an array"))?
                .iter()
                .map(dual_porosity_material)
                .collect::<Result<Vec<_>, String>>()?;
            Ok(DualPorPrimitiveState {
                coordinates: array("coordinates")?,
                old_water: array("old_water")?,
                new_pressure_head: array("new_pressure_head")?,
                old_immobile_water: array("old_immobile_water")?,
                material_ids: o
                    .get("material_ids")
                    .ok_or(format!("{name}.material_ids missing"))?
                    .as_array()
                    .ok_or(format!("{name}.material_ids must be an array"))?
                    .iter()
                    .map(|v| as_usize(v, "material_ids"))
                    .collect::<Result<Vec<_>, String>>()?,
                materials,
                critical_atmospheric_head: scalar("critical_atmospheric_head")?,
                dt: scalar("dt")?,
                hydraulic_model: as_i32(
                    o.get("hydraulic_model")
                        .ok_or(format!("{name}.hydraulic_model missing"))?,
                    "hydraulic_model",
                )?,
                dual_porosity: as_u8(
                    o.get("dual_porosity")
                        .ok_or(format!("{name}.dual_porosity missing"))?,
                    "dual_porosity",
                )?,
            })
        },
    )
}
fn maybe_biology_mode(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<BiologyModeRequest>>, String> {
    unit_value(fields, "biology_mode", "ChemIn source flags", |x| {
        let o = x
            .as_object()
            .ok_or("biology_mode value must be an object")?;
        validate_object_keys("biology_mode", o, &["iBact", "lFiltr"])?;
        Ok(BiologyModeRequest {
            i_bact: o
                .get("iBact")
                .map(|v| as_u8(v, "biology_mode.iBact"))
                .transpose()?
                .unwrap_or(0),
            filtration: o
                .get("lFiltr")
                .map(|v| as_bool(v, "biology_mode.lFiltr"))
                .transpose()?
                .unwrap_or(false),
        })
    })
}
fn maybe_biology_parameters(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<BiologyParameters>>, String> {
    unit_value(
        fields,
        "biology_parameters",
        "source ChPar slots; selector file units",
        |x| {
            let o = x
                .as_object()
                .ok_or("biology_parameters value must be an object")?;
            validate_object_keys(
                "biology_parameters",
                o,
                &[
                    "blocking_iPsi",
                    "grain_diameter",
                    "particle_diameter",
                    "attachment_capacity_1",
                    "attachment_rate_1",
                ],
            )?;
            let opt_real = |name: &str| -> Result<Option<DefaultReal>, String> {
                match o.get(name) {
                    None | Some(Value::Null) => Ok(None),
                    Some(v) => Ok(Some(DefaultReal::from_input(as_f64(v, name)?))),
                }
            };
            Ok(BiologyParameters {
                blocking_i_psi: match o.get("blocking_iPsi") {
                    None | Some(Value::Null) => None,
                    Some(v) => Some(as_u8(v, "biology_parameters.blocking_iPsi")?),
                },
                grain_diameter: opt_real("grain_diameter")?,
                particle_diameter: opt_real("particle_diameter")?,
                attachment_capacity_1: opt_real("attachment_capacity_1")?,
                attachment_rate_1: opt_real("attachment_rate_1")?,
            })
        },
    )
}

/// `lWat` and `SinkF`, the two `Site.dat` control-section logicals of the
/// `INPUT.FOR:31` read that guard the two source-coupling calls. Both keys are
/// required whenever the field is present: a request that maps one of them and
/// silently omits the other would leave the second at a Rust default instead of the
/// control-record value, and `SOLUTE.FOR:94` / `HYDRUS.FOR:390` both test two flags.
/// The result is merged into `SourceSwitches` by `derive_switches`, which is the only
/// home of these two flags; the kernels take them as parameters.
fn maybe_control_switches(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<SourceSwitches>>, String> {
    let name = "control_switches";
    unit_value(fields, name, "Site.dat control logicals", |x| {
        let o = x
            .as_object()
            .ok_or("control_switches value must be an object")?;
        validate_object_keys(name, o, &["lWat", "SinkF"])?;
        let required = |key: &str| -> Result<bool, String> {
            o.get(key)
                .map(|v| as_bool(v, &format!("{name}.{key}")))
                .ok_or_else(|| format!("{name} requires {key} from the control read"))?
        };
        Ok(SourceSwitches {
            water_flow: required("lWat")?,
            root_sink: required("SinkF")?,
            ..SourceSwitches::default()
        })
    })
}

/// The `SetSSnk` primitive bundle: every one of the 17 actuals of `SOLUTE.FOR:94-96`
/// that the routine reads, in dummy-argument order (`SINK.FOR:138-139`). All keys are
/// required, because a missing producer value has no default in `src_reference` that
/// could stand in for it: `rKM` and `cMin` are read at `INPUT.FOR:1126` only under
/// `lActRSU.and.NS.eq.1` and are never defaulted anywhere, and `INPUT.FOR:1695-1697`
/// defaults only `OmegaS`, `SPot` and `OmegaW`, i.e. three of the six scalars.
/// `cRootMax` is a scalar because the call passes the element `cRootMax(jS)`.
fn maybe_root_uptake_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<RootUptakePrimitiveState>>, String> {
    let name = "root_uptake_state";
    unit_value(fields, name, "source root uptake state", |x| {
        let o = x
            .as_object()
            .ok_or("root_uptake_state value must be an object")?;
        validate_object_keys(
            name,
            o,
            &[
                "species",
                "species_count",
                "time",
                "coordinates",
                "root_distribution",
                "root_water_sink",
                "species_concentration",
                "omega_water",
                "species_root_max_concentration",
                "lActRSU",
                "solute_stress_index",
                "potential_uptake",
                "michaelis_constant",
                "minimum_concentration",
            ],
        )?;
        let real = |key: &str| -> Result<DefaultReal, String> {
            let v = o.get(key).ok_or_else(|| format!("{name} requires {key}"))?;
            Ok(DefaultReal::from_input(as_f64(
                v,
                &format!("{name}.{key}"),
            )?))
        };
        let vector = |key: &str| -> Result<Vec<DefaultReal>, String> {
            let v = o.get(key).ok_or_else(|| format!("{name} requires {key}"))?;
            number_vec(v, &format!("{name}.{key}"))
        };
        Ok(RootUptakePrimitiveState {
            species: o
                .get("species")
                .map(|v| as_usize(v, &format!("{name}.species")))
                .transpose()?
                .ok_or_else(|| format!("{name} requires species"))?,
            species_count: o
                .get("species_count")
                .map(|v| as_usize(v, &format!("{name}.species_count")))
                .transpose()?
                .ok_or_else(|| format!("{name} requires species_count"))?,
            time: DoubleReal(as_f64(
                o.get("time")
                    .ok_or_else(|| format!("{name} requires time"))?,
                &format!("{name}.time"),
            )?),
            coordinates: vector("coordinates")?,
            root_distribution: vector("root_distribution")?,
            root_water_sink: vector("root_water_sink")?,
            species_concentration: vector("species_concentration")?,
            omega_water: real("omega_water")?,
            species_root_max_concentration: real("species_root_max_concentration")?,
            active_uptake: o
                .get("lActRSU")
                .map(|v| as_bool(v, &format!("{name}.lActRSU")))
                .transpose()?
                .ok_or_else(|| format!("{name} requires lActRSU"))?,
            solute_stress_index: real("solute_stress_index")?,
            potential_uptake: real("potential_uptake")?,
            michaelis_constant: real("michaelis_constant")?,
            minimum_concentration: real("minimum_concentration")?,
        })
    })
}

/// The `Exclusion` primitive bundle: the seven values `SOLUTE.FOR:1713-1779` actually
/// reads out of the nine actuals of `HYDRUS.FOR:390-391`, plus `NumNP`. `ThOld`,
/// `vOld` and the non-first nodes of `ThNew`/`vNew` are write-only and `Par(3,1)` is a
/// dead store whose only reader, `1770`, is a column-1 comment, so none of them is
/// accepted here; an unknown key is rejected rather than ignored.
fn maybe_size_exclusion_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<SizeExclusionPrimitiveState>>, String> {
    let name = "size_exclusion_state";
    unit_value(fields, name, "source size-exclusion state", |x| {
        let o = x
            .as_object()
            .ok_or("size_exclusion_state value must be an object")?;
        validate_object_keys(
            name,
            o,
            &[
                "nodes",
                "mobile_residual_water",
                "mobile_saturated_water",
                "mobile_n",
                "mobile_tortuosity",
                "colloid_exclusion_water_content",
                "first_node_new_water",
                "first_node_new_velocity",
            ],
        )?;
        let real = |key: &str| -> Result<DefaultReal, String> {
            let v = o.get(key).ok_or_else(|| format!("{name} requires {key}"))?;
            Ok(DefaultReal::from_input(as_f64(
                v,
                &format!("{name}.{key}"),
            )?))
        };
        Ok(SizeExclusionPrimitiveState {
            nodes: o
                .get("nodes")
                .map(|v| as_usize(v, &format!("{name}.nodes")))
                .transpose()?
                .ok_or_else(|| format!("{name} requires nodes"))?,
            mobile_residual_water: real("mobile_residual_water")?,
            mobile_saturated_water: real("mobile_saturated_water")?,
            mobile_n: real("mobile_n")?,
            mobile_tortuosity: real("mobile_tortuosity")?,
            colloid_exclusion_water_content: real("colloid_exclusion_water_content")?,
            first_node_new_water: real("first_node_new_water")?,
            first_node_new_velocity: real("first_node_new_velocity")?,
        })
    })
}

fn boundary_form(field: Option<&Value>, upper: bool) -> Result<Option<BoundaryForm>, String> {
    let Some(field) = field else {
        return Ok(None);
    };
    let code = field
        .get("value")
        .map(|v| {
            as_i32(
                v,
                if upper {
                    "boundary.upper.code"
                } else {
                    "boundary.lower.code"
                },
            )
        })
        .transpose()?;
    code.map(|code| match (upper, code) {
        (false, -1) => Ok(BoundaryForm::TypeThree),
        (false, 0) => Ok(BoundaryForm::ZeroGradient),
        (false, 1..) => Ok(BoundaryForm::Prescribed),
        (false, _) => Err("unsupported lower boundary code below -1".into()),
        (true, -2) => Ok(BoundaryForm::Atmospheric),
        (true, 1..) => Ok(BoundaryForm::Prescribed),
        (true, _) => Ok(BoundaryForm::TypeThree),
    })
    .transpose()
}

fn validate_object_keys(
    name: &str,
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), String> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("field {name}: unknown nested key {key}"));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::Path;
    fn parse_request_text(request: &str) -> ParsedRequest {
        let hash = canonical_hash(&serde_json::from_str(request).unwrap());
        parse_request(&format!(r#"{{"request":{request}}}"#), &hash).unwrap()
    }

    /// The `Site.dat` control logicals have exactly one home each, so a request cannot
    /// assert `SinkF` or `lWat` in a bundle and a different value in the control record.
    #[test]
    fn control_switches_have_a_single_home_and_require_both_logicals() {
        let parsed = parse_coupling_request(&coupling_request(control_section(false, true)));
        assert!(parsed.input.switches.root_sink);
        assert!(!parsed.input.switches.water_flow);
        // Neither logical may be omitted: the second would fall back to a Rust default
        // instead of the control-record value, and both guards test two flags.
        for (entries, expected) in [
            (vec![("SinkF", json(true))], "lWat"),
            (vec![("lWat", json(false))], "SinkF"),
        ] {
            let mut request = Map::new();
            request.insert(
                "control_switches".into(),
                field("Site.dat control logicals", &bundle(&entries, &[])),
            );
            assert!(coupling_rejection(&request).contains(expected));
        }
        // And the two flags have no other home: a bundle cannot carry them.
        let mut request = coupling_request(control_section(false, true));
        insert(
            &mut request,
            "root_uptake_state",
            &root_uptake_bundle(&[("SinkF", json(true))]),
        );
        assert!(coupling_rejection(&request).contains("unknown nested key"));
    }

    /// Each bundle is reachable through exactly one call, so a bundle whose guard cannot
    /// fire is rejected, and the reader-forced predicates (`iBact.eq.1` for `lBact`,
    /// `NS.eq.1` for `lActRSU`) are cross-checked against the control section.
    #[test]
    fn coupling_bundles_are_rejected_when_their_call_guard_cannot_fire() {
        // root_uptake_state needs SinkF, the `SOLUTE.FOR:94` guard.
        let mut request = coupling_request(control_section(false, false));
        insert(&mut request, "root_uptake_state", &root_uptake_bundle(&[]));
        assert!(coupling_rejection(&request).contains("SinkF"));
        // ... and a control section at all.
        let mut request = Map::new();
        insert(&mut request, "root_uptake_state", &root_uptake_bundle(&[]));
        assert!(coupling_rejection(&request).contains("control-section SinkF"));
        // lActRSU with a multi-species chain: `INPUT.FOR:1122` forces it false above one.
        let mut request = coupling_request(control_section(false, true));
        insert(
            &mut request,
            "root_uptake_state",
            &root_uptake_bundle(&[("lActRSU", json(true)), ("species_count", json(2))]),
        );
        assert!(coupling_rejection(&request).contains("species_count"));
        // size_exclusion_state needs iBact=1 and lWat=false (`HYDRUS.FOR:390`).
        let mut request = coupling_request(control_section(false, false));
        insert(
            &mut request,
            "size_exclusion_state",
            &size_exclusion_bundle(&[]),
        );
        assert!(coupling_rejection(&request).contains("iBact=1"));
        let mut request = coupling_request(control_section(true, false));
        insert(&mut request, "biology_mode", &biology_mode_bundle());
        insert(
            &mut request,
            "size_exclusion_state",
            &size_exclusion_bundle(&[]),
        );
        assert!(coupling_rejection(&request).contains("lWat=false"));
        // The accepted combination parses, with the guard flags held only in switches.
        let mut request = coupling_request(control_section(false, true));
        insert(&mut request, "biology_mode", &biology_mode_bundle());
        insert(&mut request, "root_uptake_state", &root_uptake_bundle(&[]));
        insert(
            &mut request,
            "size_exclusion_state",
            &size_exclusion_bundle(&[]),
        );
        let parsed = parse_coupling_request(&request);
        assert!(parsed.input.switches.root_sink);
        assert!(parsed.input.switches.bacteria);
        assert!(!parsed.input.switches.water_flow);
        let Explicit::Available(uptake) = &parsed
            .input
            .fields
            .source_coupling
            .root_uptake_state
            .as_ref()
            .unwrap()
            .value
        else {
            panic!("the root uptake bundle must be available");
        };
        assert_eq!(uptake.species_root_max_concentration, DefaultReal(1.5));
        assert!(!uptake.active_uptake);
        let Explicit::Available(exclusion) = &parsed
            .input
            .fields
            .source_coupling
            .size_exclusion_state
            .as_ref()
            .unwrap()
            .value
        else {
            panic!("the size exclusion bundle must be available");
        };
        assert_eq!(exclusion.nodes, 3);
    }

    /// The two call sites pass fixed positional lists. `SetSSnk` receives the seventeen
    /// actuals of `SOLUTE.FOR:94-96`, whose order is the dummy list of
    /// `SINK.FOR:138-139`, with `cRootMax(jS)` reaching a scalar dummy; `Exclusion`
    /// receives the eight actuals of `SOLUTE.FOR:1713-1714`.
    #[test]
    fn coupling_bundles_map_the_source_argument_lists() {
        let mut request = coupling_request(control_section(false, true));
        insert(&mut request, "biology_mode", &biology_mode_bundle());
        insert(&mut request, "root_uptake_state", &root_uptake_bundle(&[]));
        insert(
            &mut request,
            "size_exclusion_state",
            &size_exclusion_bundle(&[]),
        );
        let parsed = parse_coupling_request(&request);
        let Explicit::Available(state) = &parsed
            .input
            .fields
            .source_coupling
            .root_uptake_state
            .as_ref()
            .unwrap()
            .value
        else {
            panic!("available");
        };
        // Seventeen actuals, minus `NSD` (a dimension parameter of `Conc`) and the
        // written `SinkS`, leaves the fourteen fields of the bundle, and they are the
        // values in dummy order.
        assert_eq!(state.species, 1); // jS
        assert_eq!(state.species_count, 1); // NS
        assert_eq!(state.time, DoubleReal(3600.0)); // t, the only double dummy
        assert_eq!(state.coordinates.len(), 3); // x
        assert_eq!(state.root_distribution.len(), 3); // Beta
        assert_eq!(state.root_water_sink.len(), 3); // Sink
        assert_eq!(state.species_concentration.len(), 3); // Conc(jS,.)
        assert_eq!(state.omega_water, DefaultReal(0.8)); // OmegaW
        assert_eq!(state.species_root_max_concentration, DefaultReal(1.5)); // cRootMax(jS)
        assert!(!state.active_uptake); // lActRSU
        assert_eq!(state.solute_stress_index, DefaultReal(1.0)); // OmegaS
        assert_eq!(state.potential_uptake, DefaultReal(0.0)); // SPot
        assert_eq!(state.michaelis_constant, DefaultReal(1.0)); // rKM
        assert_eq!(state.minimum_concentration, DefaultReal(0.5)); // cMin
                                                                   // Source precision: `t` is double, every other scalar is default REAL.
        assert_eq!(std::mem::size_of_val(&state.time), 8);
        assert_eq!(std::mem::size_of_val(&state.omega_water), 4);
        let Explicit::Available(state) = &parsed
            .input
            .fields
            .source_coupling
            .size_exclusion_state
            .as_ref()
            .unwrap()
            .value
        else {
            panic!("available");
        };
        // Eight actuals: `NMat`/`NSD` are dimension parameters, `Par(3,1)` feeds only
        // the dead `1766`, and `ThOld`/`vOld` plus nodes 2..N are write-only, which is
        // why seven readable values remain and no material or species index is carried.
        assert_eq!(state.nodes, 3); // NumNP, the 1772-1777 loop bound
        assert_eq!(state.mobile_residual_water, DefaultReal(0.1)); // Par(1,1)
        assert_eq!(state.mobile_saturated_water, DefaultReal(1.0)); // Par(2,1)
        assert_eq!(state.mobile_n, DefaultReal(2.0)); // Par(4,1)
        assert_eq!(state.mobile_tortuosity, DefaultReal(0.5)); // Par(6,1)
        assert_eq!(state.colloid_exclusion_water_content, DefaultReal(0.2)); // ChPar(4,1)
        assert_eq!(state.first_node_new_water, DefaultReal(0.6)); // ThNew(1)
        assert_eq!(state.first_node_new_velocity, DefaultReal(-0.25)); // vNew(1)
    }

    /// Write-only and dead actuals are not accepted as input: `ThOld`, `vOld`, the
    /// non-first nodes of `ThNew`/`vNew`, and `Par(3,1)` have no field, and a request
    /// naming one is rejected rather than silently ignored. `SinkS` is an output.
    #[test]
    fn coupling_bundles_reject_write_only_and_dead_state() {
        for key in [
            "old_water",
            "old_velocity",
            "mobile_alpha",
            "material_index",
            "species_dimension",
        ] {
            let mut request = coupling_request(control_section(false, false));
            insert(
                &mut request,
                "size_exclusion_state",
                &size_exclusion_bundle(&[(key, json(1.0))]),
            );
            let rejection = coupling_rejection(&request);
            assert!(rejection.contains("unknown nested key"), "{rejection}");
            assert!(rejection.contains("size_exclusion_state"), "{rejection}");
        }
        for key in ["solute_sink", "i_dual_por", "water_flow", "root_sink"] {
            let mut request = coupling_request(control_section(false, true));
            insert(
                &mut request,
                "root_uptake_state",
                &root_uptake_bundle(&[(key, json(1.0))]),
            );
            let rejection = coupling_rejection(&request);
            assert!(rejection.contains("unknown nested key"), "{rejection}");
        }
    }

    /// Every read actual is required: `rKM` and `cMin` are read only under
    /// `lActRSU.and.NS.eq.1` (`INPUT.FOR:1123-1127`) and are never defaulted anywhere in
    /// `src_reference`, so no reader default may stand in for a missing value.
    #[test]
    fn coupling_bundles_require_every_read_actual() {
        for key in [
            "species",
            "species_count",
            "time",
            "coordinates",
            "root_distribution",
            "root_water_sink",
            "species_concentration",
            "omega_water",
            "species_root_max_concentration",
            "lActRSU",
            "solute_stress_index",
            "potential_uptake",
            "michaelis_constant",
            "minimum_concentration",
        ] {
            let mut request = coupling_request(control_section(false, true));
            insert(
                &mut request,
                "root_uptake_state",
                &bundle_without(&root_uptake_entries(), key),
            );
            let rejection = coupling_rejection(&request);
            assert!(rejection.contains(key), "{key}: {rejection}");
        }
        for key in [
            "nodes",
            "mobile_residual_water",
            "mobile_saturated_water",
            "mobile_n",
            "mobile_tortuosity",
            "colloid_exclusion_water_content",
            "first_node_new_water",
            "first_node_new_velocity",
        ] {
            let mut request = coupling_request(control_section(false, false));
            insert(
                &mut request,
                "size_exclusion_state",
                &bundle_without(&size_exclusion_entries(), key),
            );
            let rejection = coupling_rejection(&request);
            assert!(rejection.contains(key), "{key}: {rejection}");
        }
    }

    /// The 38 pinned decks carry no root-uptake or size-exclusion state and enable
    /// neither guard, so neither consumer is reachable from them.
    /// The fourteen read actuals of the `SOLUTE.FOR:94-96` `SetSSnk` call.
    fn root_uptake_entries() -> Vec<(&'static str, Value)> {
        vec![
            ("species", json(1)),
            ("species_count", json(1)),
            ("time", json(3600.0)),
            ("coordinates", array(&[json(0.0), json(-10.0), json(-20.0)])),
            (
                "root_distribution",
                array(&[json(1.0), json(1.0), json(1.0)]),
            ),
            (
                "root_water_sink",
                array(&[json(0.0), json(-10.0), json(-20.0)]),
            ),
            (
                "species_concentration",
                array(&[json(2.0), json(2.5), json(3.0)]),
            ),
            ("omega_water", json(0.8)),
            ("species_root_max_concentration", json(1.5)),
            ("lActRSU", json(false)),
            ("solute_stress_index", json(1.0)),
            ("potential_uptake", json(0.0)),
            ("michaelis_constant", json(1.0)),
            ("minimum_concentration", json(0.5)),
        ]
    }

    /// The seven values `SOLUTE.FOR:1713-1779` reads, plus `NumNP`, which the call
    /// passes as the loop bound of `1772-1777`.
    fn size_exclusion_entries() -> Vec<(&'static str, Value)> {
        vec![
            ("nodes", json(3)),
            ("mobile_residual_water", json(0.1)),
            ("mobile_saturated_water", json(1.0)),
            ("mobile_n", json(2.0)),
            ("mobile_tortuosity", json(0.5)),
            ("colloid_exclusion_water_content", json(0.2)),
            ("first_node_new_water", json(0.6)),
            ("first_node_new_velocity", json(-0.25)),
        ]
    }

    fn root_uptake_bundle(overrides: &[(&str, Value)]) -> Map<String, Value> {
        bundle(&root_uptake_entries(), overrides)
    }

    fn size_exclusion_bundle(overrides: &[(&str, Value)]) -> Map<String, Value> {
        bundle(&size_exclusion_entries(), overrides)
    }

    fn biology_mode_bundle() -> Map<String, Value> {
        bundle(&[("iBact", json(1)), ("lFiltr", json(false))], &[])
    }

    /// The `INPUT.FOR:31` control logicals `SinkF` and `lWat`.
    fn control_section(l_wat: bool, sink_f: bool) -> Map<String, Value> {
        bundle(&[("lWat", json(l_wat)), ("SinkF", json(sink_f))], &[])
    }

    fn coupling_request(control: Map<String, Value>) -> Map<String, Value> {
        let mut request = Map::new();
        request.insert(
            "control_switches".into(),
            field("Site.dat control logicals", &control),
        );
        request
    }

    fn insert(request: &mut Map<String, Value>, key: &str, value: &Map<String, Value>) {
        let units = match key {
            "biology_mode" => "ChemIn source flags",
            "root_uptake_state" => "source root uptake state",
            "size_exclusion_state" => "source size-exclusion state",
            _ => "Site.dat control logicals",
        };
        request.insert(key.to_string(), field(units, value));
    }

    fn field(units: &str, value: &Map<String, Value>) -> Value {
        let mut out = Map::new();
        out.insert("units".to_string(), Value::String(units.to_string()));
        out.insert("value".to_string(), Value::Object(value.clone()));
        Value::Object(out)
    }

    fn bundle(entries: &[(&str, Value)], overrides: &[(&str, Value)]) -> Map<String, Value> {
        let mut object = Map::new();
        for (key, value) in entries.iter().chain(overrides.iter()) {
            object.insert((*key).to_string(), value.clone());
        }
        object
    }

    fn bundle_without(entries: &[(&str, Value)], key: &str) -> Map<String, Value> {
        let mut object = Map::new();
        for (k, value) in entries {
            if *k != key {
                object.insert((*k).to_string(), value.clone());
            }
        }
        object
    }

    fn array(entries: &[Value]) -> Value {
        Value::Array(entries.to_vec())
    }

    fn json(value: impl Into<Value>) -> Value {
        value.into()
    }

    /// Requests must carry the version the reader dispatches on (`adapter:157-159`).
    fn with_schema(request: Map<String, Value>) -> Map<String, Value> {
        let mut envelope = Map::new();
        envelope.insert(
            "schema_version".to_string(),
            Value::String("solute-request-v1".into()),
        );
        envelope.insert("fields".to_string(), Value::Object(request));
        envelope
    }

    fn coupling_rejection(request: &Map<String, Value>) -> String {
        match parse_generated_request(request) {
            Ok(_) => panic!("expected a rejection"),
            Err(e) => e,
        }
    }

    /// Serializes a field map the way `CASE.json` does and parses it through the real
    /// entry point, so every coupling test exercises the shipped validation path.
    fn parse_generated_request(request: &Map<String, Value>) -> Result<ParsedRequest, String> {
        let text = Value::Object(with_schema(request.clone())).to_string();
        let hash = canonical_hash(&serde_json::from_str(&text).unwrap());
        parse_request(&format!(r#"{{"request":{text}}}"#), &hash)
    }

    fn parse_coupling_request(request: &Map<String, Value>) -> ParsedRequest {
        parse_generated_request(request)
            .unwrap_or_else(|e| panic!("expected acceptance, rejected: {e}"))
    }

    #[test]
    fn pinned_requests_cannot_supply_the_source_coupling_state() {
        let root = Path::new("../../reference/fortran_reference/solute/parity");
        let root = if root.exists() {
            root.to_path_buf()
        } else {
            Path::new("reference/fortran_reference/solute/parity").to_path_buf()
        };
        let mut cases = 0;
        for entry in walkdir(&root) {
            let text = std::fs::read_to_string(&entry).unwrap();
            let parsed = parse_case_manifest(&text).unwrap();
            let coupling = &parsed.input.fields.source_coupling;
            assert!(coupling.control_switches.is_none(), "{}", entry.display());
            assert!(coupling.root_uptake_state.is_none());
            assert!(coupling.size_exclusion_state.is_none());
            // And no pinned deck enables either guard, so neither call could run.
            assert!(!parsed.input.switches.root_sink);
            cases += 1;
        }
        assert_eq!(cases, 38);
    }

    #[test]
    fn all_wave3_case_requests_match_python_canonical_hash_and_map_to_typed_input() {
        let root = Path::new("../../reference/fortran_reference/solute/parity");
        let root = if root.exists() {
            root.to_path_buf()
        } else {
            Path::new("reference/fortran_reference/solute/parity").to_path_buf()
        };
        let mut count = 0;
        for entry in walkdir(&root) {
            let text = std::fs::read_to_string(&entry).unwrap();
            let parsed =
                parse_case_manifest(&text).unwrap_or_else(|e| panic!("{}: {e}", entry.display()));
            assert_eq!(parsed.input.request_sha256, parsed.request_sha256);
            assert!(parsed.input.validate_dimensions().is_ok());
            count += 1;
        }
        assert_eq!(count, 38);
    }
    fn walkdir(path: &Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(path).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                out.extend(walkdir(&p));
            } else if p.file_name().is_some_and(|n| n == "CASE.json") {
                out.push(p)
            }
        }
        out
    }
    #[test]
    fn rejects_duplicate_unknown_nonfinite_and_bad_hash_before_mapping() {
        assert!(parse_request(r#"{"request":{"request":{"schema_version":"solute-request-v1","fields":{"x":{"units":"1","value":1},"x":{"units":"1","value":2}}}}}"#, "a".repeat(64).as_str()).is_err());
        let request = r#"{"schema_version":"solute-request-v1","fields":{"mystery":{"units":"1","value":1}}}"#;
        let hash = canonical_hash(&serde_json::from_str(request).unwrap());
        assert!(parse_request(&format!(r#"{{"request":{request}}}"#), &hash)
            .unwrap_err()
            .contains("unmapped"));
        let request = r#"{"schema_version":"solute-request-v1","fields":{"temperature_history":{"units":"degC","value":NaN}}}"#;
        assert!(parse_request(&format!(r#"{{"request":{request}}}"#), &"0".repeat(64)).is_err());
    }
    #[test]
    fn rejects_dimension_units_and_accepts_reader_reachable_nonlinear_mode() {
        let bad = r#"{"schema_version":"solute-request-v1","fields":{"linearity_mode":{"units":"boolean","value":false}}}"#;
        let hash = canonical_hash(&serde_json::from_str(bad).unwrap());
        let parsed = parse_request(&format!(r#"{{"request":{bad}}}"#), &hash).unwrap();
        assert!(!parsed.input.switches.linear);
        let bad = r#"{"schema_version":"solute-normalized-request-v1","fields":{"coordinates":{"units":"cm","value":{"native":"x","canonical":"d","nodes":2}},"temperature_history":{"units":"degC","value":[20.0]}}}"#;
        let hash = canonical_hash(&serde_json::from_str(bad).unwrap());
        assert!(parse_request(&format!(r#"{{"request":{bad}}}"#), &hash)
            .unwrap_err()
            .contains("temperature_history length"));
        let bad = r#"{"schema_version":"solute-request-v1","fields":{"time.interval":{"units":"second","value":1}}}"#;
        let hash = canonical_hash(&serde_json::from_str(bad).unwrap());
        assert!(parse_request(&format!(r#"{{"request":{bad}}}"#), &hash)
            .unwrap_err()
            .contains("expected units"));
    }

    #[test]
    fn derives_source_selectors_and_primitive_endpoint_inputs() {
        let request = r#"{"schema_version":"solute-request-v1","fields":{"species_count":{"units":"count","value":1},"ordered_species":{"units":"ordered species records","value":[{"name":"S1","position":1,"chain_rate_per_hour":0.0,"initial_concentration_C_ref":1.0,"linear_equilibrium_kd_cm3_per_g":0.0}]},"biology_mode":{"units":"ChemIn source flags","value":{"iBact":0,"lFiltr":false}},"material_species_chpar":{"units":"source ChPar slots by species/material","value":[{"species":1,"material":1,"slots":[1.0,0.0,1.0,0.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]}]},"time.interval":{"units":"hour","value":1.0},"solute.flux_concentration":{"units":"boolean","value":true},"moisture_dependency":{"units":"source selector and WDep table","value":{"selector":1,"table":[1.0,0.0,0.8,0.2]}},"initialization.mass":{"units":"boolean","value":true},"initialization.equilibrium":{"units":"boolean","value":false},"initial_dissolved_concentration":{"units":"C_ref","value":[1.0]},"boundary.lower.code":{"units":"source integer code","value":1},"boundary.upper.code":{"units":"source integer code","value":1}}}"#;
        let parsed = parse_request_text(request);
        assert!(parsed.input.switches.linear);
        assert!(parsed.input.switches.flux_concentration);
        assert_eq!(parsed.input.switches.moisture_selector, 1);
        assert!(parsed.input.switches.mass_initialization);
        assert!(!parsed.input.switches.equilibrium_initialization);

        let mut nonlinear = serde_json::from_str::<Value>(request).unwrap();
        let slots = nonlinear["fields"]["material_species_chpar"]["value"][0]["slots"]
            .as_array_mut()
            .unwrap();
        slots[7] = Value::from(0.2);
        let hash = canonical_hash(&nonlinear);
        let wrapped = format!(r#"{{"request":{nonlinear},"request_sha256":"{hash}"}}"#);
        assert!(
            !parse_request(&wrapped, &hash)
                .unwrap()
                .input
                .switches
                .linear
        );
    }

    /// The smallest state both `MatSet` and the flux closure can consume: three
    /// nodes, one material, both codes prescribed.
    const ENDPOINT_BUNDLE: &str = r#""coordinates":[0.0,2.0,5.0],"old_water":[0.4,0.5,0.6],"new_water":[0.45,0.55,0.65],"old_velocity":[0.3,0.2,-0.4],"new_velocity":[0.35,0.25,-0.5],"old_matrix_velocity":[0.3,0.2,-0.4],"new_matrix_velocity":[0.35,0.25,-0.5],"old_dispersion":[0.1,0.2,0.3],"new_dispersion":[0.12,0.22,0.32],"old_retardation":[1.1,1.2,1.3],"new_retardation":[1.15,1.25,1.35],"old_concentration":[1.0,2.0,3.0],"new_accepted_concentration":[1.5,2.5,3.5],"material_immobile_water":[0.05],"source_g0":[0.01,0.02,0.03],"source_g1":[0.04,0.05,0.06],"upwind_weight":[0.2,0.3],"epsi":0.6,"dt":2.0,"alf":0.4,"material_ids":[1,1,1],"mobile_immobile":[false],"bottom_resistance":1.0,"bottom_prescribed":4.0,"top_prescribed":5.0,"t_level":2,"r_min":1e-30"#;

    /// `ChPar(6,1)` and `ChPar(10,1)`, the gas-diffusion and Henry pair of `79`.
    const ENDPOINT_CHPAR: &str = r#","source_chpar":[{"species":1,"material":1,"slots":[0.0,0.0,0.0,0.0,0.0,0.8,0.0,0.0,0.0,0.25,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]}]"#;

    const TDEP_TEN: &str = r#","source_tdep":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]"#;

    const ATMOSPHERE: &str = r#","old_temperature":15.0,"new_temperature":25.0,"top_surface_distance":3.0,"top_atmospheric_concentration":0.7"#;

    /// Wraps the primitive bundle, plus any `","key":value` extras, in a request
    /// whose `kTopCh` is `upper_code`.
    fn endpoint_request(upper_code: i32, extra: &str) -> String {
        let primitives: Value =
            serde_json::from_str(&format!("{{{ENDPOINT_BUNDLE}{extra}}}")).unwrap();
        serde_json::json!({
            "schema_version": "solute-request-v1",
            "fields": {
                "boundary.lower.code": {"units": "source integer code", "value": 1},
                "boundary.upper.code": {"units": "source integer code", "value": upper_code},
                "endpoint_primitives": {
                    "units": "source primitive endpoint state",
                    "value": primitives,
                }
            }
        })
        .to_string()
    }

    fn endpoint_state(request: &str) -> EndpointPrimitiveState {
        let parsed = parse_request_text(request);
        let value = &parsed
            .input
            .fields
            .transport
            .endpoint_state
            .as_ref()
            .unwrap()
            .value;
        match value {
            Explicit::Available(state) => state.clone(),
            Explicit::Unavailable(_) => panic!("the endpoint bundle must be available"),
        }
    }

    fn endpoint_rejection(request: &str) -> String {
        let hash = canonical_hash(&serde_json::from_str(request).unwrap());
        parse_request(
            &format!(r#"{{"request":{request},"request_sha256":"{hash}"}}"#),
            &hash,
        )
        .unwrap_err()
    }

    #[test]
    fn endpoint_primitives_are_exactly_the_state_the_source_reads() {
        let state = endpoint_state(&endpoint_request(1, &format!("{ENDPOINT_CHPAR}{TDEP_TEN}")));
        assert_eq!(state.source_species, 1);
        assert_eq!(state.t_level, 2);
        assert_eq!(state.material_ids, vec![1, 1, 1]);
        assert_eq!(state.source_tdep.as_ref().unwrap().len(), 10);
        let records = state.source_chpar.as_ref().expect("source ChPar records");
        // One record per species/material pair carries the 20 slots, because the
        // source indexes the material in the second ChPar dimension
        // (INPUT.FOR:1341-1352).
        assert_eq!(
            (
                records[0].species,
                records[0].material,
                records[0].slots.len()
            ),
            (1, 1, 20)
        );
        // Without kTopCh=-2 the atmospheric inputs stay optional (1409-1415).
        assert_eq!(state.old_temperature, None);
        assert_eq!(state.top_atmospheric_concentration, None);

        // ChPar(4,M) is one value per material (765, 771), so a three-node
        // bundle with two materials is the normal shape, not a length error.
        let two_materials = endpoint_state(&endpoint_request(
            1,
            r#","material_immobile_water":[0.05,0.0],"material_ids":[1,2,2],"mobile_immobile":[false,true]"#,
        ));
        assert_eq!(two_materials.material_immobile_water.len(), 2);
        assert_eq!(two_materials.material_ids, vec![1, 2, 2]);

        // kTopCh=-2 reads all six of them (74-84, 884-893, 917-925).
        let atmospheric = endpoint_state(&endpoint_request(
            -2,
            &format!("{ENDPOINT_CHPAR}{TDEP_TEN}{ATMOSPHERE}"),
        ));
        assert_eq!(atmospheric.old_temperature.unwrap().0, 15.0);
        assert_eq!(atmospheric.new_temperature.unwrap().0, 25.0);
        assert_eq!(
            atmospheric.top_surface_distance.unwrap().0,
            3.0,
            "dSurf is a primitive; dSurfT is not"
        );
        assert_eq!(atmospheric.top_atmospheric_concentration.unwrap().0, 0.7);
    }

    #[test]
    fn endpoint_primitives_reject_derived_and_inconsistent_state() {
        // A precomputed Henry coefficient or gas diffusion would hide the
        // ChPar/TDep/TT derivation of 79-81 from the kernel.
        for key in [
            "henry_coefficient",
            "gas_diffusion",
            "old_accepted_flow",
            "new_accepted_flow",
            "old_accepted_concentration",
        ] {
            let rejection = endpoint_rejection(&endpoint_request(1, &format!(r#","{key}":0.1"#)));
            assert!(
                rejection.contains(&format!("unknown nested key {key}")),
                "{rejection}"
            );
        }

        // TDep is one-dimensional with a 16-entry species stride, and the
        // atmospheric code reads slots 6, 9 and 10 (INPUT.FOR:1379-1390).
        assert!(endpoint_rejection(&endpoint_request(
            1,
            r#","source_tdep":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]"#
        ))
        .contains("source_tdep"));
        assert!(endpoint_rejection(&endpoint_request(
            1,
            r#","source_chpar":[{"species":1,"material":4,"slots":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]}]"#
        ))
        .contains("source_chpar"));
        assert!(
            endpoint_rejection(&endpoint_request(1, r#","source_species":0"#))
                .contains("source_species")
        );
        assert!(
            endpoint_rejection(&endpoint_request(1, r#","material_ids":[1,1,3]"#))
                .contains("material_ids")
        );
        assert!(
            endpoint_rejection(&endpoint_request(1, r#","upwind_weight":[0.2]"#))
                .contains("upwind_weight")
        );
        // TLevel is only ever 1 or 2 when Solute calls MatSet (107, 124-128).
        assert!(endpoint_rejection(&endpoint_request(1, r#","t_level":0"#)).contains("TLevel"));
        assert!(endpoint_rejection(&endpoint_request(1, r#","t_level":3"#)).contains("TLevel"));

        // The atmospheric branch has no default for any of its six inputs.
        let missing = endpoint_rejection(&endpoint_request(
            -2,
            &format!("{ENDPOINT_CHPAR}{TDEP_TEN}"),
        ));
        assert!(
            missing.contains("atmospheric endpoint requires"),
            "{missing}"
        );
        // MatNum(N) (884, 917, 207) needs the ChPar record of this species at
        // the material of the top node, not just any record.
        let no_record = endpoint_rejection(&endpoint_request(
            -2,
            r#","mobile_immobile":[false,true],"material_ids":[2,2,2],"material_immobile_water":[0.05,0.0],"source_chpar":[{"species":1,"material":1,"slots":[0.0,0.0,0.0,0.0,0.0,0.8,0.0,0.0,0.0,0.25,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]}],"source_tdep":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],"old_temperature":15.0,"new_temperature":25.0,"top_surface_distance":3.0,"top_atmospheric_concentration":0.7"#,
        ));
        assert!(no_record.contains("source ChPar record"), "{no_record}");
    }

    #[test]
    fn review_retry_contract_regressions() {
        let lower_type_three = parse_request_text(
            r#"{"schema_version":"solute-request-v1","fields":{"boundary.lower.code":{"units":"source integer code","value":-1}}}"#,
        );
        assert_eq!(
            lower_type_three.input.lower_boundary,
            Some(BoundaryForm::TypeThree)
        );
        let lower_zero = parse_request_text(
            r#"{"schema_version":"solute-request-v1","fields":{"boundary.lower.code":{"units":"source integer code","value":0}}}"#,
        );
        assert_eq!(
            lower_zero.input.lower_boundary,
            Some(BoundaryForm::ZeroGradient)
        );
        let upper_atmospheric = parse_request_text(
            r#"{"schema_version":"solute-request-v1","fields":{"boundary.upper.code":{"units":"source integer code","value":-2}}}"#,
        );
        assert_eq!(
            upper_atmospheric.input.upper_boundary,
            Some(BoundaryForm::Atmospheric)
        );
        let upper_type_three = parse_request_text(
            r#"{"schema_version":"solute-request-v1","fields":{"boundary.upper.code":{"units":"source integer code","value":0}}}"#,
        );
        assert_eq!(
            upper_type_three.input.upper_boundary,
            Some(BoundaryForm::TypeThree)
        );
        let upper_prescribed = parse_request_text(
            r#"{"schema_version":"solute-request-v1","fields":{"boundary.upper.code":{"units":"source integer code","value":1}}}"#,
        );
        assert_eq!(
            upper_prescribed.input.upper_boundary,
            Some(BoundaryForm::Prescribed)
        );
        let mobile_immobile = parse_request_text(
            r#"{"schema_version":"solute-request-v1","fields":{"equilibrium_mode":{"units":"boolean","value":false},"immobile_water_content":{"units":"cm3/cm3","value":0.1}}}"#,
        );
        assert!(mobile_immobile.input.switches.mobile_immobile);
        let bacteria = parse_request_text(
            r#"{"schema_version":"solute-request-v1","fields":{"equilibrium_mode":{"units":"boolean","value":false},"immobile_water_content":{"units":"cm3/cm3","value":0.1},"biology_mode":{"units":"ChemIn source flags","value":{"iBact":1,"lFiltr":false}}}}"#,
        );
        assert!(!bacteria.input.switches.mobile_immobile);
        assert!(bacteria.input.switches.bacteria);
        let dual = parse_request_text(
            r#"{"schema_version":"solute-request-v1","fields":{"equilibrium_mode":{"units":"boolean","value":false},"immobile_water_content":{"units":"cm3/cm3","value":0.1},"dual_domain":{"units":"source iDualPor selector","value":{"dynamic_exchange_claimed":false,"iDualPor":1,"water_model":6}}}}"#,
        );
        assert!(!dual.input.switches.mobile_immobile);
        assert_eq!(dual.input.switches.dual_domain_model, Some(6));
        let parsed = parse_request_text(
            r#"{"schema_version":"solute-normalized-request-v1","fields":{"coordinates":{"units":"cm","value":{"native":"x=-0..-250","canonical":"flow_distance=0..250","nodes":3}}}}"#,
        );
        let coords = parsed
            .input
            .fields
            .transport_properties
            .coordinates
            .unwrap();
        assert_eq!(
            coords.value,
            Explicit::Available(CoordinatesSpec {
                native: "x=-0..-250".into(),
                canonical: "flow_distance=0..250".into(),
                nodes: 3
            })
        );
        assert_eq!(
            parsed
                .input
                .fields
                .transport
                .flow_orientation
                .unwrap()
                .value,
            Explicit::Available(FlowOrientation::NegativeNative)
        );
        assert!(matches!(parsed.input.coordinates, Explicit::Unavailable(_)));
        assert_eq!(
            parsed.input.source_indices,
            Explicit::Available(SourceIndices {
                canonical_to_native: vec![1, 2, 3]
            })
        );
        let no_permutation = parse_request_text(
            r#"{"schema_version":"solute-request-v1","fields":{"linearity_mode":{"units":"boolean","value":true}}}"#,
        );
        assert!(matches!(
            no_permutation.input.source_indices,
            Explicit::Unavailable(_)
        ));
        let lower_unsupported = r#"{"schema_version":"solute-request-v1","fields":{"boundary.lower.code":{"units":"source integer code","value":-2}}}"#;
        let hash = canonical_hash(&serde_json::from_str(lower_unsupported).unwrap());
        assert!(
            parse_request(&format!(r#"{{"request":{lower_unsupported}}}"#), &hash)
                .unwrap_err()
                .contains("unsupported lower boundary")
        );
        let mixed_record = r#"{"schema_version":"solute-request-v1","fields":{"linearity_mode":{"units":"boolean","value":true,"availability":"available","reason":"mixed"}}}"#;
        let hash = canonical_hash(&serde_json::from_str(mixed_record).unwrap());
        assert!(
            parse_request(&format!(r#"{{"request":{mixed_record}}}"#), &hash)
                .unwrap_err()
                .contains("mutually exclusive")
        );
        let unknown_nested = r#"{"schema_version":"solute-normalized-request-v1","fields":{"coordinates":{"units":"cm","value":{"native":"x=-0..-250","canonical":"flow_distance=0..250","nodes":3,"x":[0,1,2]}}}}"#;
        let hash = canonical_hash(&serde_json::from_str(unknown_nested).unwrap());
        assert!(
            parse_request(&format!(r#"{{"request":{unknown_nested}}}"#), &hash)
                .unwrap_err()
                .contains("unknown nested key")
        );
        let bad_species = r#"{"schema_version":"solute-request-v1","fields":{"species_count":{"units":"count","value":2},"ordered_species":{"units":"ordered species records","value":[{"name":"S1","position":1,"chain_rate_per_hour":0.0,"initial_concentration_C_ref":1.0,"linear_equilibrium_kd_cm3_per_g":0.0}]}}}"#;
        let hash = canonical_hash(&serde_json::from_str(bad_species).unwrap());
        assert!(
            parse_request(&format!(r#"{{"request":{bad_species}}}"#), &hash)
                .unwrap_err()
                .contains("species_count")
        );

        let bad_nodes = r#"{"schema_version":"solute-normalized-request-v1","fields":{"coordinates":{"units":"cm","value":{"native":"x=-0..-250","canonical":"flow_distance=0..250","nodes":3}},"initial_dual_immobile_water_content":{"units":"cm3/cm3","value":[0.1,0.2]}}}"#;
        let hash = canonical_hash(&serde_json::from_str(bad_nodes).unwrap());
        assert!(
            parse_request(&format!(r#"{{"request":{bad_nodes}}}"#), &hash)
                .unwrap_err()
                .contains("initial_dual_immobile_water_content length")
        );

        let request = r#"{"schema_version":"solute-request-v1","fields":{"linearity_mode":{"units":"boolean","value":true}}}"#;
        let hash = canonical_hash(&serde_json::from_str(request).unwrap());
        let manifest_without_inputs =
            format!(r#"{{"request":{request},"request_sha256":"{hash}"}}"#);
        assert!(parse_case_manifest(&manifest_without_inputs)
            .unwrap_err()
            .contains("input_files"));
        let manifest_bad_hash = format!(
            r#"{{"request":{request},"request_sha256":"{hash}","input_files":{{"SELECTOR.IN":"not-a-hash"}}}}"#
        );
        assert!(parse_case_manifest(&manifest_bad_hash)
            .unwrap_err()
            .contains("input_files.SELECTOR.IN"));
    }

    /// A model-6 deck: three nodes on 0.25 cm spacing, one material carrying the
    /// `dual_model6` `ParD` column (`mobile_residual_water`/`mobile_saturated_water`
    /// are `ParD(1)`/`ParD(2)`, `mobile_tortuosity` is `ParD(6)`, and the
    /// immobile triple is `ParD(7)`/`ParD(8)`/`ParD(9)`), and the reader's own
    /// `hCritA` (`INPUT.FOR:102-103`).
    const DUAL_POROSITY_BUNDLE: &str = r#""coordinates":[0.0,0.25,0.5],"old_water":[0.3,0.3,0.3],"new_pressure_head":[-100.0,-100.0,-100.0],"old_immobile_water":[0.1,0.1,0.1],"material_ids":[1,1,1],"materials":[{"mobile_residual_water":0.08,"mobile_saturated_water":0.43,"mobile_tortuosity":0.5,"immobile_residual_water":0.05,"immobile_saturated_water":0.25,"immobile_alpha":0.1}],"critical_atmospheric_head":-1e10,"dt":0.1,"hydraulic_model":0,"dual_porosity":1"#;

    fn dual_porosity_bundle() -> Value {
        serde_json::from_str(&format!("{{{DUAL_POROSITY_BUNDLE}}}")).unwrap()
    }

    fn dual_porosity_request(bundle: Value, selector: Value) -> String {
        serde_json::json!({
            "schema_version": "solute-request-v1",
            "fields": {
                "dual_domain": {"units": "source iDualPor selector", "value": selector},
                "dual_porosity_exchange_state": {
                    "units": "source primitive dual-porosity exchange state",
                    "value": bundle,
                },
            }
        })
        .to_string()
    }

    fn dual_porosity_state(bundle: Value, selector: Value) -> DualPorPrimitiveState {
        let parsed = parse_request_text(&dual_porosity_request(bundle, selector));
        let value = &parsed
            .input
            .fields
            .nonequilibrium
            .dual_porosity_exchange_state
            .as_ref()
            .unwrap()
            .value;
        match value {
            Explicit::Available(state) => state.clone(),
            Explicit::Unavailable(_) => panic!("the exchange bundle must be available"),
        }
    }

    fn dual_porosity_rejection(request: &str) -> String {
        let hash = canonical_hash(&serde_json::from_str(request).unwrap());
        parse_request(
            &format!(r#"{{"request":{request},"request_sha256":"{hash}"}}"#),
            &hash,
        )
        .unwrap_err()
    }

    /// `"available"`, `"unavailable"` or `"absent"` for one optional field.
    fn explicit_state<T>(field: Option<&Explicit<T>>) -> String {
        match field {
            None => "absent".to_string(),
            Some(Explicit::Available(_)) => "available".to_string(),
            Some(Explicit::Unavailable(_)) => "unavailable".to_string(),
        }
    }

    #[test]
    fn dual_porosity_exchange_state_maps_the_state_the_source_reads() {
        let state = dual_porosity_state(
            dual_porosity_bundle(),
            serde_json::json!({ "iDualPor": 1, "water_model": 6 }),
        );
        assert_eq!(state.coordinates.len(), 3);
        assert_eq!(state.old_water, vec![DefaultReal(0.3); 3]);
        assert_eq!(state.new_pressure_head, vec![DefaultReal(-100.0); 3]);
        assert_eq!(state.old_immobile_water, vec![DefaultReal(0.1); 3]);
        assert_eq!(state.material_ids, vec![1, 1, 1]);
        assert_eq!(state.critical_atmospheric_head, DefaultReal(-1e10));
        assert_eq!(state.dt, DefaultReal(0.1));
        assert_eq!((state.hydraulic_model, state.dual_porosity), (0, 1));
        let material = &state.materials[0];
        assert_eq!(material.mobile_residual_water, DefaultReal(0.08));
        assert_eq!(material.mobile_saturated_water, DefaultReal(0.43));
        assert_eq!(material.mobile_tortuosity, Some(DefaultReal(0.5)));
        assert_eq!(material.immobile_residual_water, DefaultReal(0.05));
        assert_eq!(material.immobile_saturated_water, DefaultReal(0.25));
        assert_eq!(material.immobile_alpha, DefaultReal(0.1));
        // A model-6 deck fills ParD(1..9,M) only (INPUT.FOR:618, NPar=9), so the
        // ParD(10)/ParD(11) slots a model 7 deck adds stay absent.
        assert_eq!(material.immobile_n, None);
        assert_eq!(material.immobile_saturated_conductivity, None);
    }

    #[test]
    fn dual_porosity_exchange_state_rejects_unknown_missing_and_ragged_state() {
        let selector = serde_json::json!({ "iDualPor": 1, "water_model": 6 });

        // thr and ths have no key of their own: GenMat derives them from
        // ParD(1,M) and ParD(2,M) (INPUT.FOR:796-797), so the mobile pair is the
        // only spelling and a second one would be able to contradict it.
        let mut derived_pair = dual_porosity_bundle();
        derived_pair["materials"][0]["thr"] = serde_json::json!(0.08);
        assert!(
            dual_porosity_rejection(&dual_porosity_request(derived_pair, selector.clone()))
                .contains("unknown nested key thr")
        );

        // SinkIm and WTransf are results, and ThNewIm reaches ThOldIm only
        // through Update (WATFLOW.FOR:1112), so no output key belongs here.
        for output in ["sink_immobile", "transfer", "new_immobile_water"] {
            let mut bundle = dual_porosity_bundle();
            bundle[output] = serde_json::json!([0.0, 0.0, 0.0]);
            assert!(
                dual_porosity_rejection(&dual_porosity_request(bundle, selector.clone()))
                    .contains(&format!("unknown nested key {output}"))
            );
        }

        // hCritA is required and never defaulted.
        let mut without_h_crit_a = dual_porosity_bundle();
        without_h_crit_a
            .as_object_mut()
            .unwrap()
            .remove("critical_atmospheric_head");
        assert!(dual_porosity_rejection(&dual_porosity_request(
            without_h_crit_a,
            selector.clone()
        ))
        .contains("critical_atmospheric_head missing"));

        // ParD(9,M) is read by both dual models (WATFLOW.FOR:1037, 1044).
        let mut without_alpha = dual_porosity_bundle();
        without_alpha.as_object_mut().unwrap()["materials"][0]
            .as_object_mut()
            .unwrap()
            .remove("immobile_alpha");
        assert!(
            dual_porosity_rejection(&dual_porosity_request(without_alpha, selector.clone()))
                .contains("immobile_alpha missing")
        );

        // Mixed-length quantities share one wrapper, so the units are fixed.
        let mut wrong_units = serde_json::from_str::<Value>(&dual_porosity_request(
            dual_porosity_bundle(),
            selector.clone(),
        ))
        .unwrap();
        wrong_units["fields"]["dual_porosity_exchange_state"]["units"] =
            serde_json::json!("cm3/cm3");
        assert!(dual_porosity_rejection(&wrong_units.to_string()).contains("expected units"));

        // The four node arrays are the node count of the mesh's own coordinate
        // description (x, ThOld, hNew and ThOldIm in WATFLOW.FOR:1029-1031).
        let mut ragged = dual_porosity_bundle();
        ragged["old_water"] = serde_json::json!([0.3, 0.3]);
        let mut ragged_request =
            serde_json::from_str::<Value>(&dual_porosity_request(ragged, selector.clone()))
                .unwrap();
        ragged_request["fields"]["coordinates"] = serde_json::json!({
            "units": "cm",
            "value": {"native": "x=0..0.5", "canonical": "flow_distance=0..0.5", "nodes": 3},
        });
        assert!(
            dual_porosity_rejection(&ragged_request.to_string()).contains(
                "dual_porosity_exchange_state: dual-porosity exchange node arrays must all be the same length"
            )
        );

        let mut empty_materials = dual_porosity_bundle();
        empty_materials["materials"] = serde_json::json!([]);
        assert!(
            dual_porosity_rejection(&dual_porosity_request(empty_materials, selector.clone()))
                .contains("dual_porosity_exchange_state: dual-porosity exchange needs at least one ParD column")
        );

        for id in [0, 2] {
            let mut bad_ids = dual_porosity_bundle();
            bad_ids["material_ids"] = serde_json::json!([1, 1, id]);
            assert!(
                dual_porosity_rejection(&dual_porosity_request(bad_ids, selector.clone()))
                    .contains("dual_porosity_exchange_state: MatNum must index the ParD columns")
            );
        }

        let mut text_slot = dual_porosity_bundle();
        text_slot["materials"][0]["immobile_alpha"] = serde_json::json!("0.1");
        assert!(
            dual_porosity_rejection(&dual_porosity_request(text_slot, selector))
                .contains("expected finite number")
        );
    }

    #[test]
    fn dual_porosity_exchange_state_agrees_with_the_dual_domain_selector() {
        // iDualPor reaches the bundle and the guard of WATFLOW.FOR:43 from one
        // deck, so the two spellings may not diverge.
        let mismatch = dual_porosity_rejection(&dual_porosity_request(
            dual_porosity_bundle(),
            serde_json::json!({ "iDualPor": 2, "water_model": 7, "dynamic_exchange_claimed": true }),
        ));
        assert!(mismatch.contains("must match dual_domain.iDualPor"));

        // INPUT.FOR:619-624 replaces models 6 and 7 with iModel=0 before ParD is
        // read, so a bundle naming the deck model as iModel is not a state the
        // call can be given.
        let unremapped = dual_porosity_rejection(
            &dual_porosity_request(
                dual_porosity_bundle(),
                serde_json::json!({ "iDualPor": 1, "water_model": 6 }),
            )
            .replace("\"hydraulic_model\":0", "\"hydraulic_model\":6"),
        );
        assert!(unremapped.contains("reach DualPor as iModel 0"));

        // A model 7 deck fills ParD(6), ParD(10) and ParD(11) (INPUT.FOR:623);
        // the model-6 bundle above cannot serve it.
        let mut model7_bundle = dual_porosity_bundle();
        model7_bundle["dual_porosity"] = serde_json::json!(2);
        let missing_model7_slots = dual_porosity_rejection(&dual_porosity_request(
            model7_bundle.clone(),
            serde_json::json!({ "iDualPor": 2, "water_model": 7 }),
        ));
        assert!(missing_model7_slots.contains("water model 7 fills ParD(6,M), ParD(10,M)"));

        model7_bundle["materials"][0]["immobile_n"] = serde_json::json!(1.2);
        model7_bundle["materials"][0]["immobile_saturated_conductivity"] = serde_json::json!(0.01);
        let state = dual_porosity_state(
            model7_bundle,
            serde_json::json!({ "iDualPor": 2, "water_model": 7 }),
        );
        assert_eq!(state.dual_porosity, 2);
        assert_eq!(state.materials[0].immobile_n, Some(DefaultReal(1.2)));
        assert_eq!(
            state.materials[0].immobile_saturated_conductivity,
            Some(DefaultReal(0.01))
        );
    }

    /// Input blocker, recorded as a test: the pinned dual-domain requests carry
    /// the `iDualPor` selector but none of the node state `DualPor` reads, so no
    /// `dual_porosity_exchange_state` can be built from them without synthesizing
    /// values (see the 18.7.3 report).
    #[test]
    fn pinned_requests_cannot_supply_the_dual_porosity_exchange_state() {
        let root = Path::new("../../reference/fortran_reference/solute/parity");
        let root = if root.exists() {
            root.to_path_buf()
        } else {
            Path::new("reference/fortran_reference/solute/parity").to_path_buf()
        };
        let mut dual_decks = 0;
        for entry in walkdir(&root) {
            let text = std::fs::read_to_string(&entry).unwrap();
            let parsed = parse_case_manifest(&text).unwrap();
            let fields = &parsed.input.fields;
            assert!(fields.nonequilibrium.dual_porosity_exchange_state.is_none());
            if !entry.to_string_lossy().contains("nonequilibrium/dual") {
                continue;
            }
            dual_decks += 1;
            let selector = match &fields.nonequilibrium.dual_domain.as_ref().unwrap().value {
                Explicit::Available(selector) => selector.clone(),
                Explicit::Unavailable(_) => panic!("the dual decks carry a dual_domain selector"),
            };
            // The selector side of the state exists in every dual deck, and the
            // reader sets iDualPor above zero only for water models 6 and 7
            // (INPUT.FOR:619-624).
            assert_eq!(
                selector.i_dual_por > 0,
                matches!(selector.water_model, 6 | 7),
                "{entry:?}: {}",
                selector.i_dual_por
            );
            // None of the node state does: x is a range description, and ThOld,
            // hNew and ThOldIm are left to the run the case does not replay.
            assert_ne!(
                explicit_state(
                    fields
                        .transport_properties
                        .coordinates
                        .as_ref()
                        .map(|v| &v.value)
                ),
                "available"
            );
            assert_eq!(
                explicit_state(
                    fields
                        .nonequilibrium
                        .initial_water_content
                        .as_ref()
                        .map(|v| &v.value)
                ),
                "unavailable"
            );
            if selector.i_dual_por > 0 {
                assert_eq!(
                    explicit_state(
                        fields
                            .nonequilibrium
                            .initial_dual_immobile_water_content
                            .as_ref()
                            .map(|v| &v.value)
                    ),
                    "unavailable"
                );
            }
            // The decks also claim SinkIm is unreachable because
            // "WATFLOW.FOR:42-46 comments out call DualPor". That claim is not
            // what the pinned source says: 43 is an `if(iDualPor.gt.0)` and only
            // the continuation lines 44-46 are commented, i.e. the call is live
            // (see the 18.7.3 notes). A corrected fixture must trip this
            // assertion together with a corrected request.
            let sink = fields
                .nonequilibrium
                .dynamic_exchange_sink
                .as_ref()
                .unwrap()
                .value
                .unavailable_reason()
                .unwrap()
                .to_string();
            assert!(sink.contains("comments out call DualPor"), "{sink}");
        }
        assert_eq!(dual_decks, 3);
    }
}

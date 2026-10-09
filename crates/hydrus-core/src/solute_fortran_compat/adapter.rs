//! Strict serialized fixture request parsing and canonical hash verification.
use super::types::*;
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
        "dual_domain",
        "dynamic_exchange_sink",
        "biology_mode",
        "biology_parameters",
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
    request_fields.nonequilibrium.initial_water_content =
        maybe_number_vec(fields, "initial_water_content", "cm3/cm3")?;

    request_fields.biology.mode = maybe_biology_mode(fields)?;
    request_fields.biology.parameters = maybe_biology_parameters(fields)?;

    validate_cross_field_dimensions(&request_fields)?;
    let switches = derive_switches(&request_fields)?;
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
            if dual.dynamic_exchange_claimed {
                return Err(
                    "dynamic dual-domain exchange is not active on the audited source path".into(),
                );
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
    fn rejects_dimension_units_and_unreachable_modes() {
        let bad = r#"{"schema_version":"solute-request-v1","fields":{"linearity_mode":{"units":"boolean","value":false}}}"#;
        let hash = canonical_hash(&serde_json::from_str(bad).unwrap());
        assert!(parse_request(&format!(r#"{{"request":{bad}}}"#), &hash)
            .unwrap_err()
            .contains("nonlinear"));
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
}

//! Transport-property mappers and cross-field checks.
use super::fields::*;
use super::json_guard::validate_object_keys;
use super::*;
use serde_json::{Map, Value};

pub(super) fn maybe_coordinates(
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

pub(super) fn validate_dimensions(fields: &RequestFields) -> Result<(), String> {
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
    Ok(())
}

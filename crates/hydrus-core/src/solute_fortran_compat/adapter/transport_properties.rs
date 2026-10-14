//! Transport-property mappers and cross-field checks.
use super::fields::*;
use super::*;
use serde::Deserialize;
use serde_json::{Map, Value};

/// Rejects a non-object before a struct DTO sees it: serde would read a JSON
/// array as a tuple struct, where the old decoders required an object.
fn require_object(value: &Value, name: &str) -> Result<(), String> {
    if value.is_object() {
        Ok(())
    } else {
        Err(format!("field {name}: value must be an object"))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinatesDto {
    native: String,
    canonical: String,
    nodes: usize,
}

pub(super) fn maybe_coordinates(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<CoordinatesSpec>>, String> {
    let raw = decode_unit_value::<Value>(fields, "coordinates", "cm")?;
    map_available(raw, |v| {
        require_object(&v, "coordinates")?;
        let d: CoordinatesDto = serde_json::from_value(v).map_err(|e| {
            let text = e.to_string();
            // Keep the pre-serde wording for unknown keys (`validate_object_keys`).
            match text
                .strip_prefix("unknown field `")
                .and_then(|t| t.split_once('`'))
            {
                Some((key, _)) => format!("field coordinates: unknown nested key {key}"),
                None => format!("field coordinates: {text}"),
            }
        })?;
        if d.native.is_empty() {
            return Err("field coordinates.native: expected nonempty string".into());
        }
        if d.canonical.is_empty() {
            return Err("field coordinates.canonical: expected nonempty string".into());
        }
        if d.nodes == 0 {
            return Err("coordinates.nodes must be positive".into());
        }
        Ok(CoordinatesSpec {
            native: d.native,
            canonical: d.canonical,
            nodes: d.nodes,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn coords(value: Value) -> Result<Option<UnitValue<CoordinatesSpec>>, String> {
        let mut m = Map::new();
        m.insert("coordinates".into(), json!({"units": "cm", "value": value}));
        maybe_coordinates(&m)
    }

    // `a|b` passes on either: old bodies named the nested key, serde names the request field.
    fn err_has(r: Result<impl std::fmt::Debug, String>, needle: &str) {
        let e = r.expect_err("must reject");
        assert!(
            needle.split('|').any(|n| e.contains(n)),
            "{e:?} should name {needle}"
        );
    }

    #[test]
    fn coordinates_accepts_valid_and_rejects_bad_values() {
        let ok = json!({"native": "z", "canonical": "z", "nodes": 3});
        assert!(coords(ok).unwrap().is_some());
        err_has(coords(json!(null)), "coordinates");
        err_has(coords(json!(["z", "z", 3])), "coordinates");
        err_has(
            coords(json!({"native": "z", "canonical": "z", "nodes": 3.0})),
            "nodes|coordinates",
        );
        err_has(
            coords(json!({"native": "z", "canonical": "z", "nodes": 0})),
            "nodes|coordinates",
        );
        err_has(
            coords(json!({"native": "z", "canonical": "z", "nodes": null})),
            "nodes|coordinates",
        );
        err_has(
            coords(json!({"native": 7, "canonical": "z", "nodes": 3})),
            "native|coordinates",
        );
        err_has(
            coords(json!({"native": "", "canonical": "z", "nodes": 3})),
            "native|coordinates",
        );
        err_has(
            coords(json!({"native": "z", "nodes": 3})),
            "canonical|coordinates",
        );
        err_has(
            coords(json!({"native": "z", "canonical": "z", "nodes": 3, "extra": 1})),
            "extra|coordinates",
        );
        let mut m = Map::new();
        m.insert(
            "coordinates".into(),
            json!({"value": {"native": "z", "canonical": "z", "nodes": 3}}),
        );
        err_has(maybe_coordinates(&m), "units");
    }
}

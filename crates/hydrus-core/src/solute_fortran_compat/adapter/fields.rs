//! Scalar and unit-value field helpers.
use super::super::types::*;
use serde::de::{DeserializeOwned, IgnoredAny};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

pub(super) fn unit(v: &Value) -> Result<&str, String> {
    v.get("units")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "explicit units required".to_string())
}

pub(super) fn expect_units(v: &Value, expected: &str, name: &str) -> Result<(), String> {
    let got = unit(v)?;
    if got != expected {
        Err(format!(
            "field {name}: expected units {expected}, got {got}"
        ))
    } else {
        Ok(())
    }
}

pub(super) fn field_value<'a>(
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

pub(super) fn explicit<T>(
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

pub(super) fn as_f64(v: &Value, name: &str) -> Result<f64, String> {
    v.as_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| format!("field {name}: expected finite number"))
}

pub(super) fn as_i32(v: &Value, name: &str) -> Result<i32, String> {
    let n = v
        .as_i64()
        .ok_or_else(|| format!("field {name}: expected integer"))?;
    i32::try_from(n).map_err(|_| format!("field {name}: integer out of range"))
}

pub(super) fn as_usize(v: &Value, name: &str) -> Result<usize, String> {
    let n = v
        .as_u64()
        .ok_or_else(|| format!("field {name}: expected nonnegative integer"))?;
    usize::try_from(n).map_err(|_| format!("field {name}: integer out of range"))
}

pub(super) fn as_bool(v: &Value, name: &str) -> Result<bool, String> {
    v.as_bool()
        .ok_or_else(|| format!("field {name}: expected boolean"))
}

pub(super) fn as_string(v: &Value, name: &str) -> Result<String, String> {
    v.as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("field {name}: expected nonempty string"))
}

pub(super) fn number_vec(value: &Value, name: &str) -> Result<Vec<DefaultReal>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("field {name}: expected array"))?
        .iter()
        .map(|x| Ok(DefaultReal::from_input(as_f64(x, name)?)))
        .collect()
}

pub(super) fn unit_value<T>(
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

pub(super) fn maybe_number(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<DefaultReal>>, String> {
    unit_value(fields, name, units, |x| {
        Ok(DefaultReal::from_input(as_f64(x, name)?))
    })
}

pub(super) fn maybe_bool(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<bool>>, String> {
    unit_value(fields, name, units, |x| as_bool(x, name))
}

pub(super) fn maybe_i32(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<i32>>, String> {
    unit_value(fields, name, units, |x| as_i32(x, name))
}

pub(super) fn maybe_usize(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<usize>>, String> {
    unit_value(fields, name, units, |x| as_usize(x, name))
}

pub(super) fn maybe_string(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<String>>, String> {
    unit_value(fields, name, units, |x| as_string(x, name))
}

pub(super) fn maybe_number_vec(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<Vec<DefaultReal>>>, String> {
    unit_value(fields, name, units, |x| number_vec(x, name))
}

// Serde decoding. A mapper calls `decode_unit_value::<Dto>(fields, name, units)`
// (units are checked first, as `unit_value` does), then `map_available` to convert
// the DTO. In a DTO, mark every optional field `#[serde(default, deserialize_with =
// "present")]` so an explicit `null` is rejected rather than read as absent, and
// give required-with-default fields `#[serde(default = ...)]` on a non-Option type.

/// Deserializes a field only when its key exists, so `null` is never "absent".
pub(super) fn present<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, bound(deserialize = "T: Deserialize<'de>"))]
struct Wire<T> {
    units: String,
    #[serde(default = "no_value", deserialize_with = "present")]
    value: Option<T>,
    availability: Option<String>,
    reason: Option<String>,
    #[serde(rename = "provenance")]
    _provenance: Option<IgnoredAny>,
}

fn no_value<T>() -> Option<T> {
    None
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for UnitValue<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let w = Wire::<T>::deserialize(d)?;
        let value = match (w.value, w.availability, w.reason) {
            (Some(v), _, _) => Explicit::Available(v),
            (None, Some(availability), Some(reason)) => Explicit::Unavailable(UnavailableValue {
                availability,
                reason,
            }),
            _ => {
                return Err(D::Error::custom(
                    "unavailable field requires availability and reason",
                ))
            }
        };
        Ok(UnitValue {
            units: w.units,
            value,
        })
    }
}

/// Serde reads a JSON array as a positional tuple; the old `as_object` decoders
/// rejected it, so every struct DTO level is guarded before decoding.
pub(super) fn require_object(value: &Value, name: &str) -> Result<(), String> {
    if value.is_object() {
        Ok(())
    } else {
        Err(format!("field {name}: value must be an object"))
    }
}

pub(super) fn decode_unit_value<T: DeserializeOwned>(
    fields: &Map<String, Value>,
    name: &str,
    units: &str,
) -> Result<Option<UnitValue<T>>, String> {
    let Some(v) = field_value(fields, name, units)? else {
        return Ok(None);
    };
    serde_json::from_value(v.clone()).map(Some).map_err(|e| {
        let text = e.to_string();
        // Keep the pre-serde wording for unknown keys (`validate_object_keys`).
        match text
            .strip_prefix("unknown field `")
            .and_then(|t| t.split_once('`'))
        {
            Some((key, _)) => format!("field {name}: unknown nested key {key}"),
            None => format!("field {name}: {text}"),
        }
    })
}

pub(super) fn map_available<A, B>(
    v: Option<UnitValue<A>>,
    f: impl FnOnce(A) -> Result<B, String>,
) -> Result<Option<UnitValue<B>>, String> {
    v.map(|u| {
        Ok(UnitValue {
            units: u.units,
            value: match u.value {
                Explicit::Available(a) => Explicit::Available(f(a)?),
                Explicit::Unavailable(x) => Explicit::Unavailable(x),
            },
        })
    })
    .transpose()
}

pub(super) fn require_wrapped_object(
    fields: &Map<String, Value>,
    field: &str,
) -> Result<(), String> {
    match fields.get(field).and_then(|w| w.get("value")) {
        Some(v) => require_object(v, field),
        None => Ok(()),
    }
}

pub(super) fn reals(v: Vec<f64>) -> Vec<DefaultReal> {
    v.into_iter().map(DefaultReal::from_input).collect()
}

#[cfg(test)]
pub(super) fn wrap(field: &str, units: &str, v: Value) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert(
        field.into(),
        serde_json::json!({"units": units, "value": v}),
    );
    m
}

#[cfg(test)]
pub(super) fn err_has(res: Result<impl std::fmt::Debug, String>, needle: &str) {
    let e = res.expect_err("must reject");
    assert!(
        needle.split('|').any(|n| e.contains(n)),
        "{e:?} should name {needle}"
    );
}

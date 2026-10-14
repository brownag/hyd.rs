//! Scalar and unit-value field helpers.
use super::super::types::*;
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

pub(super) fn as_u8(v: &Value, name: &str) -> Result<u8, String> {
    let n = v
        .as_u64()
        .ok_or_else(|| format!("field {name}: expected nonnegative integer"))?;
    u8::try_from(n).map_err(|_| format!("field {name}: integer out of range"))
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

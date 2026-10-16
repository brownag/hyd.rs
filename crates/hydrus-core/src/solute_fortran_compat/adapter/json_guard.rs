//! Raw JSON syntax guards: duplicate keys, finite numbers, field and key validation.
use super::fields::*;
use serde_json::Value;
use std::collections::HashSet;

/// Scan JSON syntax before Value parsing so duplicate object keys cannot be overwritten.
pub(super) fn reject_duplicate_keys(text: &str) -> Result<(), String> {
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

pub(super) fn finite_tree(v: &Value) -> bool {
    match v {
        Value::Number(n) => n.as_f64().is_some_and(f64::is_finite),
        Value::Array(a) => a.iter().all(finite_tree),
        Value::Object(o) => o.values().all(finite_tree),
        _ => true,
    }
}

pub(super) fn validate_field(name: &str, v: &Value) -> Result<(), String> {
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

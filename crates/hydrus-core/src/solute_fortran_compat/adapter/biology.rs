//! Biology record mappers.
use super::fields::*;
use super::*;
use serde::Deserialize;
use serde_json::{Map, Value};

/// Serde reads a JSON array as a positional tuple; the old `as_object` decoders
/// rejected it, so every struct DTO level is guarded before decoding.
fn require_object(value: &Value, name: &str) -> Result<(), String> {
    if value.is_object() {
        Ok(())
    } else {
        Err(format!("field {name}: value must be an object"))
    }
}

fn require_wrapped_object(fields: &Map<String, Value>, name: &str) -> Result<(), String> {
    match fields.get(name).and_then(|w| w.get("value")) {
        Some(v) => require_object(v, name),
        None => Ok(()),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BiologyModeDto {
    #[serde(default, rename = "iBact")]
    i_bact: u8,
    #[serde(default, rename = "lFiltr")]
    filtration: bool,
}

pub(super) fn maybe_biology_mode(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<BiologyModeRequest>>, String> {
    require_wrapped_object(fields, "biology_mode")?;
    let dto = decode_unit_value::<BiologyModeDto>(fields, "biology_mode", "ChemIn source flags")?;
    map_available(dto, |d| {
        Ok(BiologyModeRequest {
            i_bact: d.i_bact,
            filtration: d.filtration,
        })
    })
}

// Every field here is optional and an explicit null reads as absent (the old body
// matched `None | Some(Null)`), so plain `default` on `Option` is the faithful form.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BiologyParametersDto {
    #[serde(default, rename = "blocking_iPsi")]
    blocking_i_psi: Option<u8>,
    #[serde(default)]
    grain_diameter: Option<f64>,
    #[serde(default)]
    particle_diameter: Option<f64>,
    #[serde(default)]
    attachment_capacity_1: Option<f64>,
    #[serde(default)]
    attachment_rate_1: Option<f64>,
}

pub(super) fn maybe_biology_parameters(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<BiologyParameters>>, String> {
    require_wrapped_object(fields, "biology_parameters")?;
    let dto = decode_unit_value::<BiologyParametersDto>(
        fields,
        "biology_parameters",
        "source ChPar slots; selector file units",
    )?;
    map_available(dto, |d| {
        let real = DefaultReal::from_input;
        Ok(BiologyParameters {
            blocking_i_psi: d.blocking_i_psi,
            grain_diameter: d.grain_diameter.map(real),
            particle_diameter: d.particle_diameter.map(real),
            attachment_capacity_1: d.attachment_capacity_1.map(real),
            attachment_rate_1: d.attachment_rate_1.map(real),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn wrap(name: &str, units: &str, value: Value) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert(name.into(), json!({"units": units, "value": value}));
        m
    }

    // `a|b` passes on either: old bodies named the nested key, serde names the request field.
    fn err_has(r: Result<impl std::fmt::Debug, String>, needle: &str) {
        let e = r.expect_err("must reject");
        assert!(
            needle.split('|').any(|n| e.contains(n)),
            "{e:?} should name {needle}"
        );
    }

    const MODE: &str = "ChemIn source flags";
    const PARAMS: &str = "source ChPar slots; selector file units";

    fn mode(v: Value) -> Result<Option<UnitValue<BiologyModeRequest>>, String> {
        maybe_biology_mode(&wrap("biology_mode", MODE, v))
    }

    fn params(v: Value) -> Result<Option<UnitValue<BiologyParameters>>, String> {
        maybe_biology_parameters(&wrap("biology_parameters", PARAMS, v))
    }

    #[test]
    fn accepts_defaults_and_null_parameters() {
        assert!(mode(json!({})).unwrap().is_some());
        assert!(mode(json!({"iBact": 1, "lFiltr": true})).unwrap().is_some());
        assert!(params(json!({})).unwrap().is_some());
        // Old body read an explicit null parameter as absent.
        assert!(params(json!({
            "blocking_iPsi": null, "grain_diameter": null, "particle_diameter": null,
            "attachment_capacity_1": null, "attachment_rate_1": null
        }))
        .unwrap()
        .is_some());
        assert!(params(json!({"grain_diameter": 1, "blocking_iPsi": 2}))
            .unwrap()
            .is_some());
    }

    #[test]
    fn rejects_explicit_null() {
        err_has(mode(json!({"iBact": null})), "iBact|biology_mode");
        err_has(mode(json!({"lFiltr": null})), "lFiltr|biology_mode");
        err_has(mode(json!(null)), "biology_mode");
    }

    #[test]
    fn rejects_wrong_json_type() {
        err_has(mode(json!({"lFiltr": "yes"})), "lFiltr|biology_mode");
        err_has(
            params(json!({"grain_diameter": "1"})),
            "grain_diameter|biology_parameters",
        );
    }

    #[test]
    fn rejects_float_for_integer() {
        err_has(mode(json!({"iBact": 1.0})), "iBact|biology_mode");
        err_has(
            params(json!({"blocking_iPsi": 1.0})),
            "blocking_iPsi|biology_parameters",
        );
        err_has(mode(json!({"iBact": 256})), "iBact|biology_mode");
    }

    #[test]
    fn rejects_missing_units() {
        let mut m = Map::new();
        m.insert("biology_mode".into(), json!({"value": {}}));
        err_has(maybe_biology_mode(&m), "units|biology_mode");
        let mut m = Map::new();
        m.insert("biology_parameters".into(), json!({"value": {}}));
        err_has(maybe_biology_parameters(&m), "units|biology_parameters");
    }

    #[test]
    fn rejects_unknown_nested_key() {
        err_has(mode(json!({"bogus": 1})), "bogus|biology_mode");
        err_has(params(json!({"bogus": 1})), "bogus|biology_parameters");
    }

    #[test]
    fn rejects_array_as_struct() {
        err_has(mode(json!([1, true])), "biology_mode");
        err_has(params(json!([1, 2, 3, 4, 5])), "biology_parameters");
    }
}

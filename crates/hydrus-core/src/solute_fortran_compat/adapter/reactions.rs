//! Reaction and species record mappers and cross-field checks.
use super::fields::*;
use super::*;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{Map, Value};

fn decode_object<T: DeserializeOwned>(value: &Value, name: &str) -> Result<T, String> {
    require_object(value, name)?;
    serde_json::from_value(value.clone()).map_err(|e| {
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

fn reals(name: &str, v: Vec<f64>) -> Result<Vec<DefaultReal>, String> {
    v.into_iter().map(|x| real(name, x)).collect()
}

fn real(name: &str, x: f64) -> Result<DefaultReal, String> {
    if x.is_finite() {
        Ok(DefaultReal::from_input(x))
    } else {
        Err(format!("field {name}: expected finite number"))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChparDto {
    species: usize,
    material: usize,
    slots: Vec<f64>,
}

/// `ChPar((jS-1)*16+slot, M)` records: the source reads four material slots and
/// 16 slots per species, indexing the material in the second array dimension
/// (`INPUT.FOR:1333-1352`).
pub(super) fn chpar_records(
    value: &Value,
    name: &str,
) -> Result<Vec<MaterialSpeciesParameters>, String> {
    let entries = value
        .as_array()
        .ok_or_else(|| format!("{name} value must be an array"))?;
    entries
        .iter()
        .map(|entry| {
            let d = decode_object::<ChparDto>(entry, name)?;
            let slots = reals(&format!("{name}.slots"), d.slots)?;
            if slots.len() != 20 {
                return Err(format!("{name} slots must have length 20"));
            }
            Ok(MaterialSpeciesParameters {
                species: d.species,
                material: d.material,
                slots,
            })
        })
        .collect()
}

pub(super) fn maybe_chpar(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<Vec<MaterialSpeciesParameters>>>, String> {
    unit_value(
        fields,
        "material_species_chpar",
        "source ChPar slots by species/material",
        |x| chpar_records(x, "material_species_chpar"),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MoistureDto {
    selector: u8,
    #[serde(default)]
    table: Vec<f64>,
}

pub(super) fn maybe_moisture_dependency(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<MoistureDependencyRequest>>, String> {
    let raw = decode_unit_value::<Value>(
        fields,
        "moisture_dependency",
        "source selector and WDep table",
    )?;
    map_available(raw, |v| {
        let d = decode_object::<MoistureDto>(&v, "moisture_dependency")?;
        if d.selector > 2 {
            return Err("moisture_dependency.selector must be 0, 1, or 2".into());
        }
        let table = reals("moisture_dependency.table", d.table)?;
        if d.selector == 1 && table.is_empty() {
            return Err("selector 1 requires a WDep table".into());
        }
        Ok(MoistureDependencyRequest {
            selector: d.selector,
            table,
        })
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpeciesDto {
    name: String,
    position: usize,
    chain_rate_per_hour: f64,
    #[serde(rename = "initial_concentration_C_ref")]
    initial_concentration: f64,
    #[serde(rename = "linear_equilibrium_kd_cm3_per_g")]
    linear_equilibrium_kd: f64,
}

pub(super) fn maybe_species(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<Vec<SpeciesRecord>>>, String> {
    let raw =
        decode_unit_value::<Vec<Value>>(fields, "ordered_species", "ordered species records")?;
    map_available(raw, |entries| {
        entries
            .iter()
            .map(|entry| {
                let d = decode_object::<SpeciesDto>(entry, "ordered_species")?;
                if d.name.is_empty() {
                    return Err("field ordered_species.name: expected nonempty string".into());
                }
                Ok(SpeciesRecord {
                    name: d.name,
                    position: d.position,
                    chain_rate_per_hour: real(
                        "ordered_species.chain_rate_per_hour",
                        d.chain_rate_per_hour,
                    )?,
                    initial_concentration: real(
                        "ordered_species.initial_concentration_C_ref",
                        d.initial_concentration,
                    )?,
                    linear_equilibrium_kd: real(
                        "ordered_species.linear_equilibrium_kd_cm3_per_g",
                        d.linear_equilibrium_kd,
                    )?,
                })
            })
            .collect()
    })
}

pub(super) fn validate_ordered_species(fields: &RequestFields) -> Result<(), String> {
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
    Ok(())
}

pub(super) fn validate_material_species_chpar(fields: &RequestFields) -> Result<(), String> {
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn one(name: &str, units: &str, value: Value) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert(name.into(), json!({"units": units, "value": value}));
        m
    }

    fn species(value: Value) -> Result<Option<UnitValue<Vec<SpeciesRecord>>>, String> {
        maybe_species(&one("ordered_species", "ordered species records", value))
    }

    fn chpar(value: Value) -> Result<Option<UnitValue<Vec<MaterialSpeciesParameters>>>, String> {
        maybe_chpar(&one(
            "material_species_chpar",
            "source ChPar slots by species/material",
            value,
        ))
    }

    fn moisture(value: Value) -> Result<Option<UnitValue<MoistureDependencyRequest>>, String> {
        maybe_moisture_dependency(&one(
            "moisture_dependency",
            "source selector and WDep table",
            value,
        ))
    }

    // `a|b` passes on either: old bodies named the nested key, serde names the request field.
    fn err_has(r: Result<impl std::fmt::Debug, String>, needle: &str) {
        let e = r.expect_err("must reject");
        assert!(
            needle.split('|').any(|n| e.contains(n)),
            "{e:?} should name {needle}"
        );
    }

    fn sp(patch: Value) -> Value {
        let mut v = json!({
            "name": "a", "position": 1, "chain_rate_per_hour": 0.0,
            "initial_concentration_C_ref": 1, "linear_equilibrium_kd_cm3_per_g": 0.5
        });
        for (k, x) in patch.as_object().unwrap() {
            v[k] = x.clone();
        }
        v
    }

    fn ch(patch: Value) -> Value {
        let mut v = json!({"species": 1, "material": 1, "slots": vec![0.0; 20]});
        for (k, x) in patch.as_object().unwrap() {
            v[k] = x.clone();
        }
        v
    }

    #[test]
    fn species_accepts_valid_and_rejects_bad_records() {
        assert!(species(json!([sp(json!({}))])).unwrap().is_some());
        err_has(species(json!(null)), "ordered_species");
        err_has(species(json!({"a": 1})), "ordered_species");
        err_has(species(json!([[1, 2]])), "ordered_species");
        err_has(
            species(json!([sp(json!({"position": 1.0}))])),
            "position|ordered_species",
        );
        err_has(
            species(json!([sp(json!({"name": ""}))])),
            "name|ordered_species",
        );
        err_has(
            species(json!([sp(json!({"name": 7}))])),
            "name|ordered_species",
        );
        err_has(
            species(json!([sp(json!({"extra": 1}))])),
            "extra|ordered_species",
        );
        err_has(
            species(json!([sp(json!({"chain_rate_per_hour": null}))])),
            "chain_rate_per_hour|ordered_species",
        );
        err_has(
            species(json!([{"name": "a", "position": 1}])),
            "chain_rate_per_hour|ordered_species",
        );
        let mut m = Map::new();
        m.insert("ordered_species".into(), json!({"value": [sp(json!({}))]}));
        err_has(maybe_species(&m), "units");
    }

    #[test]
    fn chpar_accepts_valid_and_rejects_bad_records() {
        assert!(chpar(json!([ch(json!({}))])).unwrap().is_some());
        err_has(chpar(json!(null)), "material_species_chpar");
        err_has(chpar(json!({"a": 1})), "material_species_chpar");
        err_has(chpar(json!([[1, 1, 2]])), "material_species_chpar");
        err_has(
            chpar(json!([ch(json!({"species": 1.0}))])),
            "species|material_species_chpar",
        );
        err_has(
            chpar(json!([ch(json!({"material": null}))])),
            "material|material_species_chpar",
        );
        err_has(
            chpar(json!([ch(json!({"slots": vec![0.0; 19]}))])),
            "slots|material_species_chpar",
        );
        err_has(
            chpar(json!([ch(json!({"slots": "x"}))])),
            "slots|material_species_chpar",
        );
        err_has(
            chpar(json!([ch(json!({"extra": 1}))])),
            "extra|material_species_chpar",
        );
        let mut m = Map::new();
        m.insert("material_species_chpar".into(), json!({"value": []}));
        err_has(maybe_chpar(&m), "units");
    }

    #[test]
    fn chpar_records_names_the_given_path() {
        let name = "endpoint_primitives.source_chpar";
        assert_eq!(
            chpar_records(&json!([ch(json!({}))]), name).unwrap().len(),
            1
        );
        err_has(chpar_records(&json!([[1]]), name), name);
        err_has(
            chpar_records(&json!([ch(json!({"species": 1.5}))]), name),
            name,
        );
        err_has(
            chpar_records(&json!([ch(json!({"extra": 1}))]), name),
            "extra",
        );
        err_has(chpar_records(&json!(null), name), name);
    }

    #[test]
    fn moisture_accepts_valid_and_rejects_bad_values() {
        assert!(moisture(json!({"selector": 0})).unwrap().is_some());
        assert!(moisture(json!({"selector": 1, "table": [1, 2.5]}))
            .unwrap()
            .is_some());
        err_has(moisture(json!(null)), "moisture_dependency");
        err_has(moisture(json!([1, []])), "moisture_dependency");
        err_has(
            moisture(json!({"selector": 1.0})),
            "selector|moisture_dependency",
        );
        err_has(
            moisture(json!({"selector": null})),
            "selector|moisture_dependency",
        );
        err_has(
            moisture(json!({"selector": 3})),
            "selector|moisture_dependency",
        );
        err_has(
            moisture(json!({"table": []})),
            "selector|moisture_dependency",
        );
        err_has(
            moisture(json!({"selector": 0, "table": null})),
            "table|moisture_dependency",
        );
        err_has(
            moisture(json!({"selector": 0, "table": ["x"]})),
            "table|moisture_dependency",
        );
        err_has(moisture(json!({"selector": 1})), "WDep|moisture_dependency");
        err_has(
            moisture(json!({"selector": 0, "extra": 1})),
            "extra|moisture_dependency",
        );
        let mut m = Map::new();
        m.insert(
            "moisture_dependency".into(),
            json!({"value": {"selector": 0}}),
        );
        err_has(maybe_moisture_dependency(&m), "units");
    }
}

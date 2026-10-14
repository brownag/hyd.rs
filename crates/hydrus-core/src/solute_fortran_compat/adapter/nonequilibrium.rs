//! Nonequilibrium record mappers and cross-field checks.
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

/// Keeps the pre-serde wording for an absent required key ("`name.key` missing")
/// and for a non-number in a real slot ("expected finite number").
fn legacy_text(name: &str, text: String) -> String {
    match text
        .split_once("missing field `")
        .and_then(|(_, t)| t.split_once('`'))
    {
        Some((key, _)) => format!("{name}.{key} missing"),
        None => text.replace(", expected f64", ", expected finite number"),
    }
}

fn reals(v: Vec<f64>) -> Vec<DefaultReal> {
    v.into_iter().map(DefaultReal::from_input).collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DualDomainDto {
    #[serde(default)]
    dynamic_exchange_claimed: bool,
    #[serde(rename = "iDualPor")]
    i_dual_por: u8,
    water_model: u8,
}

pub(super) fn maybe_dual_domain(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<DualDomainRequest>>, String> {
    require_wrapped_object(fields, "dual_domain")?;
    let dto =
        decode_unit_value::<DualDomainDto>(fields, "dual_domain", "source iDualPor selector")?;
    map_available(dto, |d| {
        Ok(DualDomainRequest {
            dynamic_exchange_claimed: d.dynamic_exchange_claimed,
            i_dual_por: d.i_dual_por,
            water_model: d.water_model,
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DualPorMaterialDto {
    mobile_residual_water: f64,
    mobile_saturated_water: f64,
    #[serde(default, deserialize_with = "present")]
    mobile_tortuosity: Option<f64>,
    immobile_residual_water: f64,
    immobile_saturated_water: f64,
    immobile_alpha: f64,
    #[serde(default, deserialize_with = "present")]
    immobile_n: Option<f64>,
    #[serde(default, deserialize_with = "present")]
    immobile_saturated_conductivity: Option<f64>,
}

impl From<DualPorMaterialDto> for DualPorMaterial {
    fn from(d: DualPorMaterialDto) -> Self {
        let real = DefaultReal::from_input;
        DualPorMaterial {
            mobile_residual_water: real(d.mobile_residual_water),
            mobile_saturated_water: real(d.mobile_saturated_water),
            mobile_tortuosity: d.mobile_tortuosity.map(real),
            immobile_residual_water: real(d.immobile_residual_water),
            immobile_saturated_water: real(d.immobile_saturated_water),
            immobile_alpha: real(d.immobile_alpha),
            immobile_n: d.immobile_n.map(real),
            immobile_saturated_conductivity: d.immobile_saturated_conductivity.map(real),
        }
    }
}

pub(super) fn dual_porosity_material(record: &Value) -> Result<DualPorMaterial, String> {
    let name = "dual_porosity_exchange_state.materials";
    require_object(record, name)?;
    serde_json::from_value::<DualPorMaterialDto>(record.clone())
        .map(DualPorMaterial::from)
        .map_err(|e| {
            let text = e.to_string();
            // Keep the pre-serde wording for unknown keys (`validate_object_keys`).
            match text
                .strip_prefix("unknown field `")
                .and_then(|t| t.split_once('`'))
            {
                Some((key, _)) => format!("field {name}: unknown nested key {key}"),
                None => legacy_text(name, format!("field {name}: {text}")),
            }
        })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DualPorStateDto {
    coordinates: Vec<f64>,
    old_water: Vec<f64>,
    new_pressure_head: Vec<f64>,
    old_immobile_water: Vec<f64>,
    material_ids: Vec<usize>,
    materials: Vec<Value>,
    critical_atmospheric_head: f64,
    dt: f64,
    hydraulic_model: i32,
    dual_porosity: u8,
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
pub(super) fn maybe_dual_porosity_exchange_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<DualPorPrimitiveState>>, String> {
    let name = "dual_porosity_exchange_state";
    require_wrapped_object(fields, name)?;
    let dto = decode_unit_value::<DualPorStateDto>(
        fields,
        name,
        "source primitive dual-porosity exchange state",
    )
    .map_err(|e| legacy_text(name, e))?;
    // `materials` stays raw so each record is object-guarded, then decoded by
    // `dual_porosity_material`.
    map_available(dto, |d| {
        Ok(DualPorPrimitiveState {
            coordinates: reals(d.coordinates),
            old_water: reals(d.old_water),
            new_pressure_head: reals(d.new_pressure_head),
            old_immobile_water: reals(d.old_immobile_water),
            material_ids: d.material_ids,
            materials: d
                .materials
                .iter()
                .map(dual_porosity_material)
                .collect::<Result<Vec<_>, String>>()?,
            critical_atmospheric_head: DefaultReal::from_input(d.critical_atmospheric_head),
            dt: DefaultReal::from_input(d.dt),
            hydraulic_model: d.hydraulic_model,
            dual_porosity: d.dual_porosity,
        })
    })
}

pub(super) fn validate_dual_domain_dimensions(fields: &RequestFields) -> Result<(), String> {
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
    Ok(())
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

    fn dual(v: Value) -> Result<Option<UnitValue<DualDomainRequest>>, String> {
        maybe_dual_domain(&wrap("dual_domain", "source iDualPor selector", v))
    }

    fn material_json() -> Value {
        json!({
            "mobile_residual_water": 0.05, "mobile_saturated_water": 0.4,
            "immobile_residual_water": 0.0, "immobile_saturated_water": 0.1,
            "immobile_alpha": 0.5
        })
    }

    fn material(patch: Value) -> Result<DualPorMaterial, String> {
        let mut v = material_json();
        for (k, x) in patch.as_object().unwrap() {
            v[k] = x.clone();
        }
        dual_porosity_material(&v)
    }

    fn state_json() -> Value {
        json!({
            "coordinates": [0.0, 1.0], "old_water": [0.3, 0.3],
            "new_pressure_head": [-10.0, -10.0], "old_immobile_water": [0.05, 0.05],
            "material_ids": [1, 1], "materials": [material_json()],
            "critical_atmospheric_head": -1.0e10, "dt": 0.1,
            "hydraulic_model": 0, "dual_porosity": 1
        })
    }

    fn state(patch: Value) -> Result<Option<UnitValue<DualPorPrimitiveState>>, String> {
        let mut v = state_json();
        for (k, x) in patch.as_object().unwrap() {
            v[k] = x.clone();
        }
        maybe_dual_porosity_exchange_state(&wrap(
            "dual_porosity_exchange_state",
            "source primitive dual-porosity exchange state",
            v,
        ))
    }

    #[test]
    fn accepts_valid_forms() {
        assert!(dual(json!({"iDualPor": 1, "water_model": 6}))
            .unwrap()
            .is_some());
        assert!(material(json!({"immobile_n": 2, "mobile_tortuosity": 0.5})).is_ok());
        assert!(state(json!({"dt": 1})).unwrap().is_some());
    }

    #[test]
    fn rejects_explicit_null() {
        err_has(
            dual(json!({"iDualPor": null, "water_model": 6})),
            "iDualPor|dual_domain",
        );
        err_has(
            dual(json!({"dynamic_exchange_claimed": null, "iDualPor": 1, "water_model": 6})),
            "dynamic_exchange_claimed|dual_domain",
        );
        err_has(
            material(json!({"immobile_n": null})),
            "immobile_n|materials",
        );
        err_has(
            material(json!({"immobile_alpha": null})),
            "immobile_alpha|materials",
        );
        err_has(
            state(json!({"dt": null})),
            "dt|dual_porosity_exchange_state",
        );
    }

    #[test]
    fn rejects_wrong_json_type() {
        err_has(
            dual(json!({"iDualPor": "1", "water_model": 6})),
            "iDualPor|dual_domain",
        );
        err_has(
            dual(json!({"dynamic_exchange_claimed": 1, "iDualPor": 1, "water_model": 6})),
            "dynamic_exchange_claimed|dual_domain",
        );
        err_has(
            material(json!({"immobile_alpha": "x"})),
            "immobile_alpha|materials",
        );
        err_has(
            state(json!({"coordinates": "x"})),
            "coordinates|dual_porosity_exchange_state",
        );
        err_has(
            state(json!({"material_ids": ["a"]})),
            "material_ids|dual_porosity_exchange_state",
        );
    }

    #[test]
    fn rejects_float_for_integer() {
        err_has(
            dual(json!({"iDualPor": 1.0, "water_model": 6})),
            "iDualPor|dual_domain",
        );
        err_has(
            dual(json!({"iDualPor": 1, "water_model": 6.0})),
            "water_model|dual_domain",
        );
        err_has(
            state(json!({"hydraulic_model": 0.0})),
            "hydraulic_model|dual_porosity_exchange_state",
        );
        err_has(
            state(json!({"dual_porosity": 1.0})),
            "dual_porosity|dual_porosity_exchange_state",
        );
        err_has(
            state(json!({"material_ids": [1.0]})),
            "material_ids|dual_porosity_exchange_state",
        );
        err_has(
            state(json!({"dual_porosity": 256})),
            "dual_porosity|dual_porosity_exchange_state",
        );
    }

    #[test]
    fn rejects_missing_units() {
        let mut m = Map::new();
        m.insert("dual_domain".into(), json!({"value": {}}));
        err_has(maybe_dual_domain(&m), "units|materials");
        let mut m = Map::new();
        m.insert("dual_porosity_exchange_state".into(), json!({"value": {}}));
        err_has(
            maybe_dual_porosity_exchange_state(&m),
            "units|dual_porosity_exchange_state",
        );
    }

    #[test]
    fn rejects_unknown_nested_key() {
        err_has(
            dual(json!({"iDualPor": 1, "water_model": 6, "bogus": 1})),
            "bogus|dual_domain",
        );
        err_has(
            state(json!({"bogus": 1})),
            "bogus|dual_porosity_exchange_state",
        );
        err_has(material(json!({"bogus": 1})), "bogus|materials");
        let mut bad = material_json();
        bad["bogus"] = json!(1);
        err_has(
            state(json!({"materials": [bad]})),
            "bogus|dual_porosity_exchange_state",
        );
    }

    #[test]
    fn rejects_missing_required_key() {
        err_has(dual(json!({"water_model": 6})), "iDualPor|dual_domain");
        err_has(dual(json!({"iDualPor": 1})), "water_model|dual_domain");
        let mut v = material_json();
        v.as_object_mut().unwrap().remove("immobile_alpha");
        err_has(dual_porosity_material(&v), "immobile_alpha|materials");
        let mut v = state_json();
        v.as_object_mut().unwrap().remove("dt");
        err_has(
            maybe_dual_porosity_exchange_state(&wrap(
                "dual_porosity_exchange_state",
                "source primitive dual-porosity exchange state",
                v,
            )),
            "dt|dual_porosity_exchange_state",
        );
    }

    #[test]
    fn rejects_array_as_struct() {
        err_has(dual(json!([1, 6])), "dual_domain");
        err_has(
            state(json!({"materials": [[0.05, 0.4, 0.0, 0.1, 0.5]]})),
            "materials|dual_porosity_exchange_state",
        );
        err_has(
            dual_porosity_material(&json!([0.05, 0.4, 0.0, 0.1, 0.5])),
            "materials",
        );
        err_has(
            maybe_dual_porosity_exchange_state(&wrap(
                "dual_porosity_exchange_state",
                "source primitive dual-porosity exchange state",
                json!([[0.0], [0.3], [-10.0], [0.05], [1], [], -1.0, 0.1, 0, 1]),
            )),
            "dual_porosity_exchange_state",
        );
    }
}

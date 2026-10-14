//! Source-coupling record mappers and guard cross-checks.
use super::super::source_coupling;
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

/// `lWat` and `SinkF`, the two `Site.dat` control-section logicals of the
/// `INPUT.FOR:31` read that guard the two source-coupling calls. Both keys are
/// required whenever the field is present: a request that maps one of them and
/// silently omits the other would leave the second at a Rust default instead of the
/// control-record value, and `SOLUTE.FOR:94` / `HYDRUS.FOR:390` both test two flags.
/// The result is merged into `SourceSwitches` by `derive_switches`, which is the only
/// home of these two flags; the kernels take them as parameters.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ControlSwitchesDto {
    #[serde(rename = "lWat")]
    water_flow: bool,
    #[serde(rename = "SinkF")]
    root_sink: bool,
}

pub(super) fn maybe_control_switches(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<SourceSwitches>>, String> {
    require_wrapped_object(fields, "control_switches")?;
    let dto = decode_unit_value::<ControlSwitchesDto>(
        fields,
        "control_switches",
        "Site.dat control logicals",
    )?;
    map_available(dto, |d| {
        Ok(SourceSwitches {
            water_flow: d.water_flow,
            root_sink: d.root_sink,
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RootUptakeDto {
    species: usize,
    species_count: usize,
    time: f64,
    coordinates: Vec<f64>,
    root_distribution: Vec<f64>,
    root_water_sink: Vec<f64>,
    species_concentration: Vec<f64>,
    omega_water: f64,
    species_root_max_concentration: f64,
    #[serde(rename = "lActRSU")]
    active_uptake: bool,
    solute_stress_index: f64,
    potential_uptake: f64,
    michaelis_constant: f64,
    minimum_concentration: f64,
}

fn reals(v: Vec<f64>) -> Vec<DefaultReal> {
    v.into_iter().map(DefaultReal::from_input).collect()
}

pub(super) fn maybe_root_uptake_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<RootUptakePrimitiveState>>, String> {
    require_wrapped_object(fields, "root_uptake_state")?;
    let dto = decode_unit_value::<RootUptakeDto>(
        fields,
        "root_uptake_state",
        "source root uptake state",
    )?;
    map_available(dto, |d| {
        let real = DefaultReal::from_input;
        Ok(RootUptakePrimitiveState {
            species: d.species,
            species_count: d.species_count,
            time: DoubleReal(d.time),
            coordinates: reals(d.coordinates),
            root_distribution: reals(d.root_distribution),
            root_water_sink: reals(d.root_water_sink),
            species_concentration: reals(d.species_concentration),
            omega_water: real(d.omega_water),
            species_root_max_concentration: real(d.species_root_max_concentration),
            active_uptake: d.active_uptake,
            solute_stress_index: real(d.solute_stress_index),
            potential_uptake: real(d.potential_uptake),
            michaelis_constant: real(d.michaelis_constant),
            minimum_concentration: real(d.minimum_concentration),
        })
    })
}

/// The `Exclusion` primitive bundle: the seven values `SOLUTE.FOR:1713-1779` actually
/// reads out of the nine actuals of `HYDRUS.FOR:390-391`, plus `NumNP`. `ThOld`,
/// `vOld` and the non-first nodes of `ThNew`/`vNew` are write-only and `Par(3,1)` is a
/// dead store whose only reader, `1770`, is a column-1 comment, so none of them is
/// accepted here; an unknown key is rejected rather than ignored.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SizeExclusionDto {
    nodes: usize,
    mobile_residual_water: f64,
    mobile_saturated_water: f64,
    mobile_n: f64,
    mobile_tortuosity: f64,
    colloid_exclusion_water_content: f64,
    first_node_new_water: f64,
    first_node_new_velocity: f64,
}

pub(super) fn maybe_size_exclusion_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<SizeExclusionPrimitiveState>>, String> {
    require_wrapped_object(fields, "size_exclusion_state")?;
    let dto = decode_unit_value::<SizeExclusionDto>(
        fields,
        "size_exclusion_state",
        "source size-exclusion state",
    )?;
    map_available(dto, |d| {
        let real = DefaultReal::from_input;
        Ok(SizeExclusionPrimitiveState {
            nodes: d.nodes,
            mobile_residual_water: real(d.mobile_residual_water),
            mobile_saturated_water: real(d.mobile_saturated_water),
            mobile_n: real(d.mobile_n),
            mobile_tortuosity: real(d.mobile_tortuosity),
            colloid_exclusion_water_content: real(d.colloid_exclusion_water_content),
            first_node_new_water: real(d.first_node_new_water),
            first_node_new_velocity: real(d.first_node_new_velocity),
        })
    })
}

pub(super) fn validate_source_coupling_guards(
    fields: &RequestFields,
    switches: &SourceSwitches,
) -> Result<(), String> {
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

    fn control(v: Value) -> Result<Option<UnitValue<SourceSwitches>>, String> {
        maybe_control_switches(&wrap("control_switches", "Site.dat control logicals", v))
    }

    fn uptake_json() -> Value {
        json!({
            "species": 1, "species_count": 1, "time": 1.0, "coordinates": [0.0, 1.0],
            "root_distribution": [1.0, 1.0], "root_water_sink": [0.0, 0.0],
            "species_concentration": [1.0, 1.0], "omega_water": 1.0,
            "species_root_max_concentration": 1.0, "lActRSU": false,
            "solute_stress_index": 1.0, "potential_uptake": 1.0,
            "michaelis_constant": 1.0, "minimum_concentration": 0.0
        })
    }

    fn uptake(patch: Value) -> Result<Option<UnitValue<RootUptakePrimitiveState>>, String> {
        let mut v = uptake_json();
        for (k, x) in patch.as_object().unwrap() {
            v[k] = x.clone();
        }
        maybe_root_uptake_state(&wrap("root_uptake_state", "source root uptake state", v))
    }

    fn exclusion_json() -> Value {
        json!({
            "nodes": 2, "mobile_residual_water": 0.1, "mobile_saturated_water": 0.4,
            "mobile_n": 1.5, "mobile_tortuosity": 0.5,
            "colloid_exclusion_water_content": 0.3, "first_node_new_water": 0.2,
            "first_node_new_velocity": 0.0
        })
    }

    fn exclusion(patch: Value) -> Result<Option<UnitValue<SizeExclusionPrimitiveState>>, String> {
        let mut v = exclusion_json();
        for (k, x) in patch.as_object().unwrap() {
            v[k] = x.clone();
        }
        maybe_size_exclusion_state(&wrap(
            "size_exclusion_state",
            "source size-exclusion state",
            v,
        ))
    }

    #[test]
    fn accepts_valid_forms() {
        assert!(control(json!({"lWat": true, "SinkF": false}))
            .unwrap()
            .is_some());
        assert!(uptake(json!({"time": 2})).unwrap().is_some());
        assert!(exclusion(json!({})).unwrap().is_some());
    }

    #[test]
    fn rejects_explicit_null() {
        err_has(
            control(json!({"lWat": null, "SinkF": true})),
            "lWat|control_switches",
        );
        err_has(
            uptake(json!({"omega_water": null})),
            "omega_water|root_uptake_state",
        );
        err_has(
            uptake(json!({"lActRSU": null})),
            "lActRSU|root_uptake_state",
        );
        err_has(
            exclusion(json!({"nodes": null})),
            "nodes|size_exclusion_state",
        );
    }

    #[test]
    fn rejects_wrong_json_type() {
        err_has(
            control(json!({"lWat": "true", "SinkF": true})),
            "lWat|control_switches",
        );
        err_has(uptake(json!({"time": "1"})), "time|root_uptake_state");
        err_has(
            uptake(json!({"coordinates": 1.0})),
            "coordinates|root_uptake_state",
        );
        err_has(
            exclusion(json!({"mobile_n": true})),
            "mobile_n|size_exclusion_state",
        );
    }

    #[test]
    fn rejects_float_for_integer() {
        err_has(uptake(json!({"species": 1.0})), "species|root_uptake_state");
        err_has(
            uptake(json!({"species_count": -1})),
            "species_count|root_uptake_state",
        );
        err_has(
            exclusion(json!({"nodes": 2.0})),
            "nodes|size_exclusion_state",
        );
    }

    #[test]
    fn rejects_missing_units() {
        let only = |name: &str| {
            let mut m = Map::new();
            m.insert(name.into(), json!({"value": {}}));
            m
        };
        err_has(
            maybe_control_switches(&only("control_switches")),
            "units|control_switches",
        );
        err_has(
            maybe_root_uptake_state(&only("root_uptake_state")),
            "units|root_uptake_state",
        );
        err_has(
            maybe_size_exclusion_state(&only("size_exclusion_state")),
            "units|size_exclusion_state",
        );
    }

    #[test]
    fn rejects_unknown_nested_key() {
        err_has(
            control(json!({"lWat": true, "SinkF": true, "x": 1})),
            "x|control_switches",
        );
        err_has(uptake(json!({"bogus": 1})), "bogus|root_uptake_state");
        err_has(exclusion(json!({"bogus": 1})), "bogus|size_exclusion_state");
    }

    #[test]
    fn rejects_missing_required_key() {
        err_has(control(json!({"lWat": true})), "SinkF|control_switches");
        let mut v = uptake_json();
        v.as_object_mut().unwrap().remove("potential_uptake");
        err_has(
            maybe_root_uptake_state(&wrap("root_uptake_state", "source root uptake state", v)),
            "potential_uptake|root_uptake_state",
        );
        let mut v = exclusion_json();
        v.as_object_mut().unwrap().remove("first_node_new_water");
        err_has(
            maybe_size_exclusion_state(&wrap(
                "size_exclusion_state",
                "source size-exclusion state",
                v,
            )),
            "first_node_new_water|size_exclusion_state",
        );
    }

    #[test]
    fn rejects_array_as_struct() {
        err_has(control(json!([true, false])), "control_switches");
        err_has(
            maybe_root_uptake_state(&wrap(
                "root_uptake_state",
                "source root uptake state",
                json!([1, 1, 1.0]),
            )),
            "root_uptake_state",
        );
        err_has(
            maybe_size_exclusion_state(&wrap(
                "size_exclusion_state",
                "source size-exclusion state",
                json!([2, 0.1, 0.4, 1.5, 0.5, 0.3, 0.2, 0.0]),
            )),
            "size_exclusion_state",
        );
    }
}

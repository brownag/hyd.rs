//! Transport record mappers and cross-field checks.
use super::fields::*;
use super::reactions::chpar_records;
use super::*;
use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EndpointDto {
    coordinates: Vec<f64>,
    old_water: Vec<f64>,
    new_water: Vec<f64>,
    old_velocity: Vec<f64>,
    new_velocity: Vec<f64>,
    old_dispersion: Vec<f64>,
    new_dispersion: Vec<f64>,
    old_retardation: Vec<f64>,
    new_retardation: Vec<f64>,
    old_concentration: Vec<f64>,
    material_immobile_water: Vec<f64>,
    old_source_g0: Vec<f64>,
    old_source_g1: Vec<f64>,
    new_source_g0: Vec<f64>,
    new_source_g1: Vec<f64>,
    upwind_weight: Vec<f64>,
    epsi: f64,
    dt: f64,
    alf: f64,
    #[serde(default, deserialize_with = "present")]
    top_atmospheric_concentration: Option<f64>,
    #[serde(default, deserialize_with = "present")]
    top_surface_distance: Option<f64>,
    #[serde(default, deserialize_with = "present")]
    old_temperature: Option<f64>,
    #[serde(default, deserialize_with = "present")]
    new_temperature: Option<f64>,
    #[serde(default, deserialize_with = "present")]
    source_chpar: Option<Value>,
    #[serde(default, deserialize_with = "present")]
    source_tdep: Option<Vec<f64>>,
    material_ids: Vec<usize>,
    old_matrix_velocity: Vec<f64>,
    new_matrix_velocity: Vec<f64>,
    mobile_immobile: Vec<bool>,
    #[serde(default = "one")]
    source_species: usize,
    #[serde(default)]
    bacteria: bool,
    #[serde(default)]
    dual_porosity: u8,
    bottom_resistance: f64,
    bottom_prescribed: f64,
    top_prescribed: f64,
    new_accepted_concentration: Vec<f64>,
    #[serde(default)]
    t_level: i32,
    #[serde(default)]
    vapor: bool,
    r_min: f64,
}

fn one() -> usize {
    1
}

impl TryFrom<EndpointDto> for EndpointPrimitiveState {
    type Error = String;
    fn try_from(d: EndpointDto) -> Result<Self, String> {
        let real = DefaultReal::from_input;
        Ok(EndpointPrimitiveState {
            coordinates: reals(d.coordinates),
            old_water: reals(d.old_water),
            new_water: reals(d.new_water),
            old_velocity: reals(d.old_velocity),
            new_velocity: reals(d.new_velocity),
            old_dispersion: reals(d.old_dispersion),
            new_dispersion: reals(d.new_dispersion),
            old_retardation: reals(d.old_retardation),
            new_retardation: reals(d.new_retardation),
            old_concentration: reals(d.old_concentration),
            material_immobile_water: reals(d.material_immobile_water),
            old_source_g0: reals(d.old_source_g0),
            old_source_g1: reals(d.old_source_g1),
            new_source_g0: reals(d.new_source_g0),
            new_source_g1: reals(d.new_source_g1),
            upwind_weight: reals(d.upwind_weight),
            epsi: real(d.epsi),
            dt: real(d.dt),
            alf: real(d.alf),
            top_atmospheric_concentration: d.top_atmospheric_concentration.map(real),
            top_surface_distance: d.top_surface_distance.map(real),
            old_temperature: d.old_temperature.map(real),
            new_temperature: d.new_temperature.map(real),
            source_chpar: d
                .source_chpar
                .as_ref()
                .map(|v| chpar_records(v, "endpoint_primitives.source_chpar"))
                .transpose()?,
            source_tdep: d.source_tdep.map(reals),
            material_ids: d.material_ids,
            mobile_immobile: d.mobile_immobile,
            source_species: d.source_species,
            bacteria: d.bacteria,
            dual_porosity: d.dual_porosity,
            bottom_resistance: real(d.bottom_resistance),
            bottom_prescribed: real(d.bottom_prescribed),
            top_prescribed: real(d.top_prescribed),
            new_accepted_concentration: reals(d.new_accepted_concentration),
            old_matrix_velocity: reals(d.old_matrix_velocity),
            new_matrix_velocity: reals(d.new_matrix_velocity),
            t_level: d.t_level,
            vapor: d.vapor,
            r_min: real(d.r_min),
        })
    }
}

pub(super) fn maybe_endpoint_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<EndpointPrimitiveState>>, String> {
    require_wrapped_object(fields, "endpoint_primitives")?;
    let dto = decode_unit_value::<EndpointDto>(
        fields,
        "endpoint_primitives",
        "source primitive endpoint state",
    )?;
    map_available(dto, EndpointPrimitiveState::try_from)
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum OrientationDto {
    PositiveNative,
    NegativeNative,
}

pub(super) fn maybe_orientation(
    fields: &Map<String, Value>,
    name: &str,
) -> Result<Option<UnitValue<FlowOrientation>>, String> {
    let dto = decode_unit_value::<OrientationDto>(fields, name, "orientation")?;
    map_available(dto, |o| {
        Ok(match o {
            OrientationDto::PositiveNative => FlowOrientation::PositiveNative,
            OrientationDto::NegativeNative => FlowOrientation::NegativeNative,
        })
    })
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct SwitchesDto {
    #[serde(rename = "lArtD")]
    art_d: bool,
    #[serde(rename = "lTort")]
    tort: bool,
    // Accepted for compatibility; the value is not read.
    #[serde(rename = "iTort")]
    _i_tort: Option<IgnoredAny>,
    #[serde(rename = "lTDep")]
    t_dep: bool,
    #[serde(rename = "lVapor")]
    vapor: bool,
    gas_terms: bool,
    #[serde(rename = "lUpW")]
    up_w: bool,
}

pub(super) fn maybe_transport_switches(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<SourceSwitches>>, String> {
    require_wrapped_object(fields, "transport_switches")?;
    let dto = decode_unit_value::<SwitchesDto>(fields, "transport_switches", "source flags")?;
    map_available(dto, |d| {
        Ok(SourceSwitches {
            upstream: d.up_w,
            artificial_dispersion: d.art_d,
            tortuosity: d.tort,
            temperature: d.t_dep,
            vapor: d.vapor,
            gas_terms: d.gas_terms,
            ..SourceSwitches::default()
        })
    })
}

pub(super) fn boundary_form(
    field: Option<&Value>,
    upper: bool,
) -> Result<Option<BoundaryForm>, String> {
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

pub(super) fn validate_endpoint_state(fields: &RequestFields) -> Result<(), String> {
    if let Some(state) = &fields.transport.endpoint_state {
        if let Explicit::Available(s) = &state.value {
            // Shape, range and finiteness live in the kernel. The codes are only
            // read when both are available; `validate_required_mode_inputs` rejects
            // a missing code, so the placeholder 0 only keeps the shape checks on.
            let code = |c: &Option<UnitValue<i32>>| match c.as_ref().map(|c| &c.value) {
                Some(Explicit::Available(code)) => *code,
                _ => 0,
            };
            boundary::validate(
                s,
                code(&fields.transport.lower_boundary_code),
                code(&fields.transport.upper_boundary_code),
            )
            .map_err(|e| format!("endpoint_primitives: {e}"))?;
            // TDep is one-dimensional with a 16-entry stride per species
            // (INPUT.FOR:1379-1390); the atmospheric code reads slots 6, 9, 10.
            let species_base = (s.source_species - 1) * 16;
            if s.source_tdep
                .as_ref()
                .is_some_and(|v| v.len() < species_base + 10)
            {
                return Err("endpoint_primitives.source_tdep must cover TDep((jS-1)*16+10)".into());
            }
            if s.source_chpar.iter().flatten().any(|record| {
                record.species == 0
                    || record.material == 0
                    || record.material > s.mobile_immobile.len()
            }) {
                return Err(
                    "endpoint_primitives.source_chpar must name a known species and material"
                        .into(),
                );
            }
            if let Some(code) = fields.transport.upper_boundary_code.as_ref() {
                if matches!(&code.value, Explicit::Available(-2)) {
                    let complete = s.source_chpar.is_some()
                        && s.source_tdep.is_some()
                        && s.old_temperature.is_some()
                        && s.new_temperature.is_some()
                        && s.top_surface_distance.is_some()
                        && s.top_atmospheric_concentration.is_some();
                    if !complete {
                        return Err("atmospheric endpoint requires ChPar/TDep, both temperatures, surface distance, and atmospheric concentration".into());
                    }
                    // MatNum(N) (884, 917, 207) is the only material the endpoint
                    // atmosphere reads.
                    let top_material = s.material_ids.last().copied().unwrap_or(0);
                    if !s.source_chpar.as_ref().is_some_and(|records| {
                        records
                            .iter()
                            .any(|r| r.species == s.source_species && r.material == top_material)
                    }) {
                        return Err("atmospheric endpoint requires a source ChPar record for the species at the top node material".into());
                    }
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

    fn endpoint(patch: Value) -> Map<String, Value> {
        let a = json!([1.0, 2.0]);
        let mut v = json!({
            "coordinates": a, "old_water": a, "new_water": a, "old_velocity": a,
            "new_velocity": a, "old_dispersion": a, "new_dispersion": a,
            "old_retardation": a, "new_retardation": a, "old_concentration": a,
            "material_immobile_water": a, "old_source_g0": a, "old_source_g1": a,
            "new_source_g0": a, "new_source_g1": a,
            "upwind_weight": a, "epsi": 0.5, "dt": 1.0, "alf": 0.0,
            "material_ids": [1, 1], "mobile_immobile": [false],
            "bottom_resistance": 0.0, "bottom_prescribed": 0.0, "top_prescribed": 0.0,
            "new_accepted_concentration": a, "old_matrix_velocity": a,
            "new_matrix_velocity": a, "r_min": 1.0
        });
        for (k, x) in patch.as_object().unwrap() {
            v[k] = x.clone();
        }
        let mut m = Map::new();
        m.insert(
            "endpoint_primitives".into(),
            json!({"units": "source primitive endpoint state", "value": v}),
        );
        m
    }

    fn switches(value: Value) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert(
            "transport_switches".into(),
            json!({"units": "source flags", "value": value}),
        );
        m
    }

    #[test]
    fn accepts_valid_and_ignored_forms() {
        let m = endpoint(json!({}));
        assert!(maybe_endpoint_state(&m).unwrap().is_some());
        let m = endpoint(json!({"source_species": 2, "t_level": -1}));
        assert!(maybe_endpoint_state(&m).unwrap().is_some());
        let mut m = switches(json!({"lUpW": true, "iTort": "anything"}));
        m.get_mut("transport_switches").unwrap()["provenance"] = json!("x");
        assert!(maybe_transport_switches(&m).unwrap().is_some());
        let mut o = Map::new();
        o.insert(
            "o".into(),
            json!({"units": "orientation", "value": "negative-native"}),
        );
        assert!(maybe_orientation(&o, "o").unwrap().is_some());
    }

    #[test]
    fn rejects_explicit_null() {
        err_has(
            maybe_transport_switches(&switches(json!({"lUpW": null}))),
            "lUpW|transport_switches",
        );
        err_has(
            maybe_endpoint_state(&endpoint(json!({"t_level": null}))),
            "t_level|endpoint_primitives",
        );
        let mut o = Map::new();
        o.insert("o".into(), json!({"units": "orientation", "value": null}));
        err_has(maybe_orientation(&o, "o"), "o");
    }

    #[test]
    fn rejects_wrong_json_type() {
        err_has(
            maybe_transport_switches(&switches(json!({"lArtD": "yes"}))),
            "lArtD|transport_switches",
        );
        err_has(
            maybe_endpoint_state(&endpoint(json!({"epsi": "1"}))),
            "epsi|endpoint_primitives",
        );
    }

    #[test]
    fn rejects_float_for_integer() {
        err_has(
            maybe_endpoint_state(&endpoint(json!({"source_species": 3.0}))),
            "source_species|endpoint_primitives",
        );
        err_has(
            maybe_endpoint_state(&endpoint(json!({"t_level": 1.5}))),
            "t_level|endpoint_primitives",
        );
    }

    #[test]
    fn rejects_missing_units() {
        let mut m = Map::new();
        m.insert("transport_switches".into(), json!({"value": {}}));
        err_has(maybe_transport_switches(&m), "units");
    }

    #[test]
    fn rejects_unknown_nested_key() {
        err_has(
            maybe_transport_switches(&switches(json!({"bogus": true}))),
            "bogus",
        );
        err_has(
            maybe_endpoint_state(&endpoint(json!({"bogus": 1}))),
            "bogus",
        );
    }

    #[test]
    fn rejects_missing_required_and_bad_orientation() {
        let mut m = endpoint(json!({}));
        m.get_mut("endpoint_primitives").unwrap()["value"]
            .as_object_mut()
            .unwrap()
            .remove("epsi");
        err_has(maybe_endpoint_state(&m), "epsi");
        let mut o = Map::new();
        o.insert("o".into(), json!({"units": "orientation", "value": ""}));
        err_has(maybe_orientation(&o, "o"), "field o");
    }

    #[test]
    fn rejects_each_missing_level_g_pair_member() {
        for key in [
            "old_source_g0",
            "old_source_g1",
            "new_source_g0",
            "new_source_g1",
        ] {
            let mut m = endpoint(json!({}));
            m.get_mut("endpoint_primitives").unwrap()["value"]
                .as_object_mut()
                .unwrap()
                .remove(key);
            err_has(maybe_endpoint_state(&m), key);
        }
        // The pre-split single pair is no longer a known key.
        let m = endpoint(json!({"source_g0": [1.0, 2.0]}));
        assert!(maybe_endpoint_state(&m).is_err());
    }

    #[test]
    fn rejects_array_as_endpoint_struct() {
        // A complete positional tuple in field order: serde would accept it.
        let a = json!([1.0, 2.0]);
        let mut positional = vec![a.clone(); 14];
        positional.extend([json!(0.5), json!(1.0), json!(0.0)]);
        positional.extend([
            json!(1.0),
            json!(1.0),
            json!(1.0),
            json!(1.0),
            json!([]),
            a.clone(),
        ]);
        positional.extend([json!([1, 1]), a.clone(), a.clone(), json!([false])]);
        positional.extend([json!(1), json!(false), json!(0)]);
        positional.extend([json!(0.0), json!(0.0), json!(0.0), a]);
        positional.extend([json!(0), json!(false), json!(1.0)]);
        let positional = Value::Array(positional);
        let mut m = Map::new();
        m.insert(
            "endpoint_primitives".into(),
            json!({"units": "source primitive endpoint state", "value": positional}),
        );
        err_has(maybe_endpoint_state(&m), "endpoint_primitives");
    }

    #[test]
    fn rejects_array_as_switches_struct() {
        err_has(
            maybe_transport_switches(&switches(json!([true, false]))),
            "transport_switches",
        );
    }

    #[test]
    fn rejects_empty_array_as_switches_struct() {
        err_has(
            maybe_transport_switches(&switches(json!([]))),
            "transport_switches",
        );
    }
}

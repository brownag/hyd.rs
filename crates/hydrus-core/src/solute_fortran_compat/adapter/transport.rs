//! Transport record mappers and cross-field checks.
use super::fields::*;
use super::json_guard::validate_object_keys;
use super::*;
use serde_json::{Map, Value};

pub(super) fn maybe_endpoint_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<EndpointPrimitiveState>>, String> {
    unit_value(
        fields,
        "endpoint_primitives",
        "source primitive endpoint state",
        |x| {
            let o = x
                .as_object()
                .ok_or("endpoint_primitives value must be an object")?;
            let array = |name: &str| -> Result<Vec<DefaultReal>, String> {
                number_vec(
                    o.get(name)
                        .ok_or_else(|| format!("endpoint_primitives.{name} missing"))?,
                    name,
                )
            };
            let scalar = |name: &str| -> Result<DefaultReal, String> {
                Ok(DefaultReal::from_input(as_f64(
                    o.get(name)
                        .ok_or_else(|| format!("endpoint_primitives.{name} missing"))?,
                    name,
                )?))
            };
            validate_object_keys(
                "endpoint_primitives",
                o,
                &[
                    "coordinates",
                    "old_water",
                    "new_water",
                    "old_velocity",
                    "new_velocity",
                    "old_dispersion",
                    "new_dispersion",
                    "old_retardation",
                    "new_retardation",
                    "old_concentration",
                    "material_immobile_water",
                    "source_g0",
                    "source_g1",
                    "upwind_weight",
                    "epsi",
                    "dt",
                    "alf",
                    "top_atmospheric_concentration",
                    "top_surface_distance",
                    "old_temperature",
                    "new_temperature",
                    "source_chpar",
                    "source_tdep",
                    "material_ids",
                    "old_matrix_velocity",
                    "new_matrix_velocity",
                    "mobile_immobile",
                    "source_species",
                    "bacteria",
                    "dual_porosity",
                    "bottom_resistance",
                    "bottom_prescribed",
                    "top_prescribed",
                    "new_accepted_concentration",
                    "t_level",
                    "vapor",
                    "r_min",
                ],
            )?;
            let opt_real = |name: &str| -> Result<Option<DefaultReal>, String> {
                o.get(name)
                    .map(|v| as_f64(v, name).map(DefaultReal::from_input))
                    .transpose()
            };
            let source_chpar = o
                .get("source_chpar")
                .map(|v| chpar_records(v, "endpoint_primitives.source_chpar"))
                .transpose()?;
            let opt_vec = |name: &str| -> Result<Option<Vec<DefaultReal>>, String> {
                o.get(name).map(|v| number_vec(v, name)).transpose()
            };
            let usize_vec = |name: &str| -> Result<Vec<usize>, String> {
                o.get(name)
                    .ok_or_else(|| format!("endpoint_primitives.{name} missing"))?
                    .as_array()
                    .ok_or_else(|| format!("endpoint_primitives.{name} must be an array"))?
                    .iter()
                    .map(|v| as_usize(v, name))
                    .collect()
            };
            let bool_vec = |name: &str| -> Result<Vec<bool>, String> {
                o.get(name)
                    .ok_or_else(|| format!("endpoint_primitives.{name} missing"))?
                    .as_array()
                    .ok_or_else(|| format!("endpoint_primitives.{name} must be an array"))?
                    .iter()
                    .map(|v| as_bool(v, name))
                    .collect()
            };
            Ok(EndpointPrimitiveState {
                coordinates: array("coordinates")?,
                old_water: array("old_water")?,
                new_water: array("new_water")?,
                old_velocity: array("old_velocity")?,
                new_velocity: array("new_velocity")?,
                old_dispersion: array("old_dispersion")?,
                new_dispersion: array("new_dispersion")?,
                old_retardation: array("old_retardation")?,
                new_retardation: array("new_retardation")?,
                old_concentration: array("old_concentration")?,
                material_immobile_water: array("material_immobile_water")?,
                source_g0: array("source_g0")?,
                source_g1: array("source_g1")?,
                upwind_weight: array("upwind_weight")?,
                epsi: scalar("epsi")?,
                dt: scalar("dt")?,
                alf: scalar("alf")?,
                top_atmospheric_concentration: opt_real("top_atmospheric_concentration")?,
                top_surface_distance: opt_real("top_surface_distance")?,
                old_temperature: opt_real("old_temperature")?,
                new_temperature: opt_real("new_temperature")?,
                source_chpar,
                source_tdep: opt_vec("source_tdep")?,
                material_ids: usize_vec("material_ids")?,
                mobile_immobile: bool_vec("mobile_immobile")?,
                source_species: o
                    .get("source_species")
                    .map(|v| as_usize(v, "source_species"))
                    .transpose()?
                    .unwrap_or(1),
                bacteria: o
                    .get("bacteria")
                    .map(|v| as_bool(v, "bacteria"))
                    .transpose()?
                    .unwrap_or(false),
                dual_porosity: o
                    .get("dual_porosity")
                    .map(|v| as_u8(v, "dual_porosity"))
                    .transpose()?
                    .unwrap_or(0),
                bottom_resistance: scalar("bottom_resistance")?,
                bottom_prescribed: scalar("bottom_prescribed")?,
                top_prescribed: scalar("top_prescribed")?,
                new_accepted_concentration: array("new_accepted_concentration")?,
                old_matrix_velocity: array("old_matrix_velocity")?,
                new_matrix_velocity: array("new_matrix_velocity")?,
                t_level: o
                    .get("t_level")
                    .map(|v| as_i32(v, "t_level"))
                    .transpose()?
                    .unwrap_or(0),
                vapor: o
                    .get("vapor")
                    .map(|v| as_bool(v, "endpoint_primitives.vapor"))
                    .transpose()?
                    .unwrap_or(false),
                r_min: scalar("r_min")?,
            })
        },
    )
}

pub(super) fn maybe_orientation(
    fields: &Map<String, Value>,
    name: &str,
) -> Result<Option<UnitValue<FlowOrientation>>, String> {
    unit_value(fields, name, "orientation", |x| {
        match as_string(x, name)?.as_str() {
            "positive-native" => Ok(FlowOrientation::PositiveNative),
            "negative-native" => Ok(FlowOrientation::NegativeNative),
            _ => Err("boundary.flow_orientation must be positive-native or negative-native".into()),
        }
    })
}

pub(super) fn maybe_transport_switches(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<SourceSwitches>>, String> {
    unit_value(fields, "transport_switches", "source flags", |x| {
        let obj = x
            .as_object()
            .ok_or("transport_switches value must be an object")?;
        validate_object_keys(
            "transport_switches",
            obj,
            &[
                "lArtD",
                "lTort",
                "iTort",
                "lTDep",
                "lVapor",
                "gas_terms",
                "lUpW",
            ],
        )?;
        let flag = |n: &str| -> Result<bool, String> {
            Ok(obj
                .get(n)
                .map(|v| as_bool(v, n))
                .transpose()?
                .unwrap_or(false))
        };
        Ok(SourceSwitches {
            upstream: flag("lUpW")?,
            artificial_dispersion: flag("lArtD")?,
            tortuosity: flag("lTort")?,
            temperature: flag("lTDep")?,
            vapor: flag("lVapor")?,
            gas_terms: flag("gas_terms")?,
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

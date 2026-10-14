//! Nonequilibrium record mappers and cross-field checks.
use super::fields::*;
use super::json_guard::validate_object_keys;
use super::*;
use serde_json::{Map, Value};

pub(super) fn maybe_dual_domain(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<DualDomainRequest>>, String> {
    unit_value(fields, "dual_domain", "source iDualPor selector", |x| {
        let o = x.as_object().ok_or("dual_domain value must be an object")?;
        validate_object_keys(
            "dual_domain",
            o,
            &["dynamic_exchange_claimed", "iDualPor", "water_model"],
        )?;
        Ok(DualDomainRequest {
            dynamic_exchange_claimed: o
                .get("dynamic_exchange_claimed")
                .map(|v| as_bool(v, "dual_domain.dynamic_exchange_claimed"))
                .transpose()?
                .unwrap_or(false),
            i_dual_por: o
                .get("iDualPor")
                .map(|v| as_u8(v, "dual_domain.iDualPor"))
                .transpose()?
                .ok_or("dual_domain.iDualPor missing")?,
            water_model: o
                .get("water_model")
                .map(|v| as_u8(v, "dual_domain.water_model"))
                .transpose()?
                .ok_or("dual_domain.water_model missing")?,
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
pub(super) fn dual_porosity_material(record: &Value) -> Result<DualPorMaterial, String> {
    let name = "dual_porosity_exchange_state.materials";
    let o = record
        .as_object()
        .ok_or(format!("{name} records must be objects"))?;
    validate_object_keys(
        name,
        o,
        &[
            "mobile_residual_water",
            "mobile_saturated_water",
            "mobile_tortuosity",
            "immobile_residual_water",
            "immobile_saturated_water",
            "immobile_alpha",
            "immobile_n",
            "immobile_saturated_conductivity",
        ],
    )?;
    let required = |slot: &str| -> Result<DefaultReal, String> {
        Ok(DefaultReal::from_input(as_f64(
            o.get(slot)
                .ok_or_else(|| format!("{name}.{slot} missing"))?,
            slot,
        )?))
    };
    let optional = |slot: &str| -> Result<Option<DefaultReal>, String> {
        o.get(slot)
            .map(|v| as_f64(v, slot).map(DefaultReal::from_input))
            .transpose()
    };
    Ok(DualPorMaterial {
        mobile_residual_water: required("mobile_residual_water")?,
        mobile_saturated_water: required("mobile_saturated_water")?,
        mobile_tortuosity: optional("mobile_tortuosity")?,
        immobile_residual_water: required("immobile_residual_water")?,
        immobile_saturated_water: required("immobile_saturated_water")?,
        immobile_alpha: required("immobile_alpha")?,
        immobile_n: optional("immobile_n")?,
        immobile_saturated_conductivity: optional("immobile_saturated_conductivity")?,
    })
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
    unit_value(
        fields,
        name,
        "source primitive dual-porosity exchange state",
        |x| {
            let o = x
                .as_object()
                .ok_or(format!("{name} value must be an object"))?;
            validate_object_keys(
                name,
                o,
                &[
                    "coordinates",
                    "old_water",
                    "new_pressure_head",
                    "old_immobile_water",
                    "material_ids",
                    "materials",
                    "critical_atmospheric_head",
                    "dt",
                    "hydraulic_model",
                    "dual_porosity",
                ],
            )?;
            let array = |field: &str| -> Result<Vec<DefaultReal>, String> {
                number_vec(
                    o.get(field)
                        .ok_or_else(|| format!("{name}.{field} missing"))?,
                    field,
                )
            };
            let scalar = |field: &str| -> Result<DefaultReal, String> {
                Ok(DefaultReal::from_input(as_f64(
                    o.get(field)
                        .ok_or_else(|| format!("{name}.{field} missing"))?,
                    field,
                )?))
            };
            let materials = o
                .get("materials")
                .ok_or(format!("{name}.materials missing"))?
                .as_array()
                .ok_or(format!("{name}.materials must be an array"))?
                .iter()
                .map(dual_porosity_material)
                .collect::<Result<Vec<_>, String>>()?;
            Ok(DualPorPrimitiveState {
                coordinates: array("coordinates")?,
                old_water: array("old_water")?,
                new_pressure_head: array("new_pressure_head")?,
                old_immobile_water: array("old_immobile_water")?,
                material_ids: o
                    .get("material_ids")
                    .ok_or(format!("{name}.material_ids missing"))?
                    .as_array()
                    .ok_or(format!("{name}.material_ids must be an array"))?
                    .iter()
                    .map(|v| as_usize(v, "material_ids"))
                    .collect::<Result<Vec<_>, String>>()?,
                materials,
                critical_atmospheric_head: scalar("critical_atmospheric_head")?,
                dt: scalar("dt")?,
                hydraulic_model: as_i32(
                    o.get("hydraulic_model")
                        .ok_or(format!("{name}.hydraulic_model missing"))?,
                    "hydraulic_model",
                )?,
                dual_porosity: as_u8(
                    o.get("dual_porosity")
                        .ok_or(format!("{name}.dual_porosity missing"))?,
                    "dual_porosity",
                )?,
            })
        },
    )
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

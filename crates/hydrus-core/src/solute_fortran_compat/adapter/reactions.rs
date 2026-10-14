//! Reaction and species record mappers and cross-field checks.
use super::fields::*;
use super::json_guard::validate_object_keys;
use super::*;
use serde_json::{Map, Value};

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
            let o = entry
                .as_object()
                .ok_or_else(|| format!("{name} entries must be objects"))?;
            validate_object_keys(name, o, &["species", "material", "slots"])?;
            let species = o
                .get("species")
                .map(|v| as_usize(v, &format!("{name}.species")))
                .transpose()?
                .ok_or_else(|| format!("{name}.species missing"))?;
            let material = o
                .get("material")
                .map(|v| as_usize(v, &format!("{name}.material")))
                .transpose()?
                .ok_or_else(|| format!("{name}.material missing"))?;
            let slots = number_vec(
                o.get("slots")
                    .ok_or_else(|| format!("{name}.slots missing"))?,
                &format!("{name}.slots"),
            )?;
            if slots.len() != 20 {
                return Err(format!("{name} slots must have length 20"));
            }
            Ok(MaterialSpeciesParameters {
                species,
                material,
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

pub(super) fn maybe_moisture_dependency(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<MoistureDependencyRequest>>, String> {
    unit_value(
        fields,
        "moisture_dependency",
        "source selector and WDep table",
        |x| {
            let o = x
                .as_object()
                .ok_or("moisture_dependency value must be an object")?;
            validate_object_keys("moisture_dependency", o, &["selector", "table"])?;
            let selector = o
                .get("selector")
                .map(|v| as_u8(v, "moisture_dependency.selector"))
                .transpose()?
                .ok_or("moisture_dependency.selector missing")?;
            if selector > 2 {
                return Err("moisture_dependency.selector must be 0, 1, or 2".into());
            }
            let table = o
                .get("table")
                .map(|v| number_vec(v, "moisture_dependency.table"))
                .transpose()?
                .unwrap_or_default();
            if selector == 1 && table.is_empty() {
                return Err("selector 1 requires a WDep table".into());
            }
            Ok(MoistureDependencyRequest { selector, table })
        },
    )
}

pub(super) fn maybe_species(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<Vec<SpeciesRecord>>>, String> {
    unit_value(fields, "ordered_species", "ordered species records", |x| {
        x.as_array()
            .ok_or("ordered_species value must be an array")?
            .iter()
            .map(|entry| {
                let o = entry
                    .as_object()
                    .ok_or("ordered_species entries must be objects")?;
                validate_object_keys(
                    "ordered_species",
                    o,
                    &[
                        "name",
                        "position",
                        "chain_rate_per_hour",
                        "initial_concentration_C_ref",
                        "linear_equilibrium_kd_cm3_per_g",
                    ],
                )?;
                Ok(SpeciesRecord {
                    name: o
                        .get("name")
                        .map(|v| as_string(v, "ordered_species.name"))
                        .transpose()?
                        .ok_or("ordered_species.name missing")?,
                    position: o
                        .get("position")
                        .map(|v| as_usize(v, "ordered_species.position"))
                        .transpose()?
                        .ok_or("ordered_species.position missing")?,
                    chain_rate_per_hour: DefaultReal::from_input(as_f64(
                        o.get("chain_rate_per_hour")
                            .ok_or("ordered_species.chain_rate_per_hour missing")?,
                        "ordered_species.chain_rate_per_hour",
                    )?),
                    initial_concentration: DefaultReal::from_input(as_f64(
                        o.get("initial_concentration_C_ref")
                            .ok_or("ordered_species.initial_concentration_C_ref missing")?,
                        "ordered_species.initial_concentration_C_ref",
                    )?),
                    linear_equilibrium_kd: DefaultReal::from_input(as_f64(
                        o.get("linear_equilibrium_kd_cm3_per_g")
                            .ok_or("ordered_species.linear_equilibrium_kd_cm3_per_g missing")?,
                        "ordered_species.linear_equilibrium_kd_cm3_per_g",
                    )?),
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

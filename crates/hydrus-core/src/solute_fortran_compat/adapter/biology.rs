//! Biology record mappers.
use super::fields::*;
use super::json_guard::validate_object_keys;
use super::*;
use serde_json::{Map, Value};

pub(super) fn maybe_biology_mode(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<BiologyModeRequest>>, String> {
    unit_value(fields, "biology_mode", "ChemIn source flags", |x| {
        let o = x
            .as_object()
            .ok_or("biology_mode value must be an object")?;
        validate_object_keys("biology_mode", o, &["iBact", "lFiltr"])?;
        Ok(BiologyModeRequest {
            i_bact: o
                .get("iBact")
                .map(|v| as_u8(v, "biology_mode.iBact"))
                .transpose()?
                .unwrap_or(0),
            filtration: o
                .get("lFiltr")
                .map(|v| as_bool(v, "biology_mode.lFiltr"))
                .transpose()?
                .unwrap_or(false),
        })
    })
}

pub(super) fn maybe_biology_parameters(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<BiologyParameters>>, String> {
    unit_value(
        fields,
        "biology_parameters",
        "source ChPar slots; selector file units",
        |x| {
            let o = x
                .as_object()
                .ok_or("biology_parameters value must be an object")?;
            validate_object_keys(
                "biology_parameters",
                o,
                &[
                    "blocking_iPsi",
                    "grain_diameter",
                    "particle_diameter",
                    "attachment_capacity_1",
                    "attachment_rate_1",
                ],
            )?;
            let opt_real = |name: &str| -> Result<Option<DefaultReal>, String> {
                match o.get(name) {
                    None | Some(Value::Null) => Ok(None),
                    Some(v) => Ok(Some(DefaultReal::from_input(as_f64(v, name)?))),
                }
            };
            Ok(BiologyParameters {
                blocking_i_psi: match o.get("blocking_iPsi") {
                    None | Some(Value::Null) => None,
                    Some(v) => Some(as_u8(v, "biology_parameters.blocking_iPsi")?),
                },
                grain_diameter: opt_real("grain_diameter")?,
                particle_diameter: opt_real("particle_diameter")?,
                attachment_capacity_1: opt_real("attachment_capacity_1")?,
                attachment_rate_1: opt_real("attachment_rate_1")?,
            })
        },
    )
}

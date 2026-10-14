//! Source-coupling record mappers and guard cross-checks.
use super::super::source_coupling;
use super::fields::*;
use super::json_guard::validate_object_keys;
use super::*;
use serde_json::{Map, Value};

/// `lWat` and `SinkF`, the two `Site.dat` control-section logicals of the
/// `INPUT.FOR:31` read that guard the two source-coupling calls. Both keys are
/// required whenever the field is present: a request that maps one of them and
/// silently omits the other would leave the second at a Rust default instead of the
/// control-record value, and `SOLUTE.FOR:94` / `HYDRUS.FOR:390` both test two flags.
/// The result is merged into `SourceSwitches` by `derive_switches`, which is the only
/// home of these two flags; the kernels take them as parameters.
pub(super) fn maybe_control_switches(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<SourceSwitches>>, String> {
    let name = "control_switches";
    unit_value(fields, name, "Site.dat control logicals", |x| {
        let o = x
            .as_object()
            .ok_or("control_switches value must be an object")?;
        validate_object_keys(name, o, &["lWat", "SinkF"])?;
        let required = |key: &str| -> Result<bool, String> {
            o.get(key)
                .map(|v| as_bool(v, &format!("{name}.{key}")))
                .ok_or_else(|| format!("{name} requires {key} from the control read"))?
        };
        Ok(SourceSwitches {
            water_flow: required("lWat")?,
            root_sink: required("SinkF")?,
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
pub(super) fn maybe_root_uptake_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<RootUptakePrimitiveState>>, String> {
    let name = "root_uptake_state";
    unit_value(fields, name, "source root uptake state", |x| {
        let o = x
            .as_object()
            .ok_or("root_uptake_state value must be an object")?;
        validate_object_keys(
            name,
            o,
            &[
                "species",
                "species_count",
                "time",
                "coordinates",
                "root_distribution",
                "root_water_sink",
                "species_concentration",
                "omega_water",
                "species_root_max_concentration",
                "lActRSU",
                "solute_stress_index",
                "potential_uptake",
                "michaelis_constant",
                "minimum_concentration",
            ],
        )?;
        let real = |key: &str| -> Result<DefaultReal, String> {
            let v = o.get(key).ok_or_else(|| format!("{name} requires {key}"))?;
            Ok(DefaultReal::from_input(as_f64(
                v,
                &format!("{name}.{key}"),
            )?))
        };
        let vector = |key: &str| -> Result<Vec<DefaultReal>, String> {
            let v = o.get(key).ok_or_else(|| format!("{name} requires {key}"))?;
            number_vec(v, &format!("{name}.{key}"))
        };
        Ok(RootUptakePrimitiveState {
            species: o
                .get("species")
                .map(|v| as_usize(v, &format!("{name}.species")))
                .transpose()?
                .ok_or_else(|| format!("{name} requires species"))?,
            species_count: o
                .get("species_count")
                .map(|v| as_usize(v, &format!("{name}.species_count")))
                .transpose()?
                .ok_or_else(|| format!("{name} requires species_count"))?,
            time: DoubleReal(as_f64(
                o.get("time")
                    .ok_or_else(|| format!("{name} requires time"))?,
                &format!("{name}.time"),
            )?),
            coordinates: vector("coordinates")?,
            root_distribution: vector("root_distribution")?,
            root_water_sink: vector("root_water_sink")?,
            species_concentration: vector("species_concentration")?,
            omega_water: real("omega_water")?,
            species_root_max_concentration: real("species_root_max_concentration")?,
            active_uptake: o
                .get("lActRSU")
                .map(|v| as_bool(v, &format!("{name}.lActRSU")))
                .transpose()?
                .ok_or_else(|| format!("{name} requires lActRSU"))?,
            solute_stress_index: real("solute_stress_index")?,
            potential_uptake: real("potential_uptake")?,
            michaelis_constant: real("michaelis_constant")?,
            minimum_concentration: real("minimum_concentration")?,
        })
    })
}

/// The `Exclusion` primitive bundle: the seven values `SOLUTE.FOR:1713-1779` actually
/// reads out of the nine actuals of `HYDRUS.FOR:390-391`, plus `NumNP`. `ThOld`,
/// `vOld` and the non-first nodes of `ThNew`/`vNew` are write-only and `Par(3,1)` is a
/// dead store whose only reader, `1770`, is a column-1 comment, so none of them is
/// accepted here; an unknown key is rejected rather than ignored.
pub(super) fn maybe_size_exclusion_state(
    fields: &Map<String, Value>,
) -> Result<Option<UnitValue<SizeExclusionPrimitiveState>>, String> {
    let name = "size_exclusion_state";
    unit_value(fields, name, "source size-exclusion state", |x| {
        let o = x
            .as_object()
            .ok_or("size_exclusion_state value must be an object")?;
        validate_object_keys(
            name,
            o,
            &[
                "nodes",
                "mobile_residual_water",
                "mobile_saturated_water",
                "mobile_n",
                "mobile_tortuosity",
                "colloid_exclusion_water_content",
                "first_node_new_water",
                "first_node_new_velocity",
            ],
        )?;
        let real = |key: &str| -> Result<DefaultReal, String> {
            let v = o.get(key).ok_or_else(|| format!("{name} requires {key}"))?;
            Ok(DefaultReal::from_input(as_f64(
                v,
                &format!("{name}.{key}"),
            )?))
        };
        Ok(SizeExclusionPrimitiveState {
            nodes: o
                .get("nodes")
                .map(|v| as_usize(v, &format!("{name}.nodes")))
                .transpose()?
                .ok_or_else(|| format!("{name} requires nodes"))?,
            mobile_residual_water: real("mobile_residual_water")?,
            mobile_saturated_water: real("mobile_saturated_water")?,
            mobile_n: real("mobile_n")?,
            mobile_tortuosity: real("mobile_tortuosity")?,
            colloid_exclusion_water_content: real("colloid_exclusion_water_content")?,
            first_node_new_water: real("first_node_new_water")?,
            first_node_new_velocity: real("first_node_new_velocity")?,
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

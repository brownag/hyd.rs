use super::*;
use std::path::Path;
fn parse_request_text(request: &str) -> ParsedRequest {
    let hash = canonical_hash(&serde_json::from_str(request).unwrap());
    parse_request(&format!(r#"{{"request":{request}}}"#), &hash).unwrap()
}

/// The `Site.dat` control logicals have exactly one home each, so a request cannot
/// assert `SinkF` or `lWat` in a bundle and a different value in the control record.
#[test]
fn control_switches_have_a_single_home_and_require_both_logicals() {
    let parsed = parse_coupling_request(&coupling_request(control_section(false, true)));
    assert!(parsed.input.switches.root_sink);
    assert!(!parsed.input.switches.water_flow);
    // Neither logical may be omitted: the second would fall back to a Rust default
    // instead of the control-record value, and both guards test two flags.
    for (entries, expected) in [
        (vec![("SinkF", json(true))], "lWat"),
        (vec![("lWat", json(false))], "SinkF"),
    ] {
        let mut request = Map::new();
        request.insert(
            "control_switches".into(),
            field("Site.dat control logicals", &bundle(&entries, &[])),
        );
        assert!(coupling_rejection(&request).contains(expected));
    }
    // And the two flags have no other home: a bundle cannot carry them.
    let mut request = coupling_request(control_section(false, true));
    insert(
        &mut request,
        "root_uptake_state",
        &root_uptake_bundle(&[("SinkF", json(true))]),
    );
    assert!(coupling_rejection(&request).contains("unknown nested key"));
}

/// Each bundle is reachable through exactly one call, so a bundle whose guard cannot
/// fire is rejected, and the reader-forced predicates (`iBact.eq.1` for `lBact`,
/// `NS.eq.1` for `lActRSU`) are cross-checked against the control section.
#[test]
fn coupling_bundles_are_rejected_when_their_call_guard_cannot_fire() {
    // root_uptake_state needs SinkF, the `SOLUTE.FOR:94` guard.
    let mut request = coupling_request(control_section(false, false));
    insert(&mut request, "root_uptake_state", &root_uptake_bundle(&[]));
    assert!(coupling_rejection(&request).contains("SinkF"));
    // ... and a control section at all.
    let mut request = Map::new();
    insert(&mut request, "root_uptake_state", &root_uptake_bundle(&[]));
    assert!(coupling_rejection(&request).contains("control-section SinkF"));
    // lActRSU with a multi-species chain: `INPUT.FOR:1122` forces it false above one.
    let mut request = coupling_request(control_section(false, true));
    insert(
        &mut request,
        "root_uptake_state",
        &root_uptake_bundle(&[("lActRSU", json(true)), ("species_count", json(2))]),
    );
    assert!(coupling_rejection(&request).contains("species_count"));
    // size_exclusion_state needs iBact=1 and lWat=false (`HYDRUS.FOR:390`).
    let mut request = coupling_request(control_section(false, false));
    insert(
        &mut request,
        "size_exclusion_state",
        &size_exclusion_bundle(&[]),
    );
    assert!(coupling_rejection(&request).contains("iBact=1"));
    let mut request = coupling_request(control_section(true, false));
    insert(&mut request, "biology_mode", &biology_mode_bundle());
    insert(
        &mut request,
        "size_exclusion_state",
        &size_exclusion_bundle(&[]),
    );
    assert!(coupling_rejection(&request).contains("lWat=false"));
    // The accepted combination parses, with the guard flags held only in switches.
    let mut request = coupling_request(control_section(false, true));
    insert(&mut request, "biology_mode", &biology_mode_bundle());
    insert(&mut request, "root_uptake_state", &root_uptake_bundle(&[]));
    insert(
        &mut request,
        "size_exclusion_state",
        &size_exclusion_bundle(&[]),
    );
    let parsed = parse_coupling_request(&request);
    assert!(parsed.input.switches.root_sink);
    assert!(parsed.input.switches.bacteria);
    assert!(!parsed.input.switches.water_flow);
    let Explicit::Available(uptake) = &parsed
        .input
        .fields
        .source_coupling
        .root_uptake_state
        .as_ref()
        .unwrap()
        .value
    else {
        panic!("the root uptake bundle must be available");
    };
    assert_eq!(uptake.species_root_max_concentration, DefaultReal(1.5));
    assert!(!uptake.active_uptake);
    let Explicit::Available(exclusion) = &parsed
        .input
        .fields
        .source_coupling
        .size_exclusion_state
        .as_ref()
        .unwrap()
        .value
    else {
        panic!("the size exclusion bundle must be available");
    };
    assert_eq!(exclusion.nodes, 3);
}

/// The two call sites pass fixed positional lists. `SetSSnk` receives the seventeen
/// actuals of `SOLUTE.FOR:94-96`, whose order is the dummy list of
/// `SINK.FOR:138-139`, with `cRootMax(jS)` reaching a scalar dummy; `Exclusion`
/// receives the eight actuals of `SOLUTE.FOR:1713-1714`.
#[test]
fn coupling_bundles_map_the_source_argument_lists() {
    let mut request = coupling_request(control_section(false, true));
    insert(&mut request, "biology_mode", &biology_mode_bundle());
    insert(&mut request, "root_uptake_state", &root_uptake_bundle(&[]));
    insert(
        &mut request,
        "size_exclusion_state",
        &size_exclusion_bundle(&[]),
    );
    let parsed = parse_coupling_request(&request);
    let Explicit::Available(state) = &parsed
        .input
        .fields
        .source_coupling
        .root_uptake_state
        .as_ref()
        .unwrap()
        .value
    else {
        panic!("available");
    };
    // Seventeen actuals, minus `NSD` (a dimension parameter of `Conc`) and the
    // written `SinkS`, leaves the fourteen fields of the bundle, and they are the
    // values in dummy order.
    assert_eq!(state.species, 1); // jS
    assert_eq!(state.species_count, 1); // NS
    assert_eq!(state.time, DoubleReal(3600.0)); // t, the only double dummy
    assert_eq!(state.coordinates.len(), 3); // x
    assert_eq!(state.root_distribution.len(), 3); // Beta
    assert_eq!(state.root_water_sink.len(), 3); // Sink
    assert_eq!(state.species_concentration.len(), 3); // Conc(jS,.)
    assert_eq!(state.omega_water, DefaultReal(0.8)); // OmegaW
    assert_eq!(state.species_root_max_concentration, DefaultReal(1.5)); // cRootMax(jS)
    assert!(!state.active_uptake); // lActRSU
    assert_eq!(state.solute_stress_index, DefaultReal(1.0)); // OmegaS
    assert_eq!(state.potential_uptake, DefaultReal(0.0)); // SPot
    assert_eq!(state.michaelis_constant, DefaultReal(1.0)); // rKM
    assert_eq!(state.minimum_concentration, DefaultReal(0.5)); // cMin
                                                               // Source precision: `t` is double, every other scalar is default REAL.
    assert_eq!(std::mem::size_of_val(&state.time), 8);
    assert_eq!(std::mem::size_of_val(&state.omega_water), 4);
    let Explicit::Available(state) = &parsed
        .input
        .fields
        .source_coupling
        .size_exclusion_state
        .as_ref()
        .unwrap()
        .value
    else {
        panic!("available");
    };
    // Eight actuals: `NMat`/`NSD` are dimension parameters, `Par(3,1)` feeds only
    // the dead `1766`, and `ThOld`/`vOld` plus nodes 2..N are write-only, which is
    // why seven readable values remain and no material or species index is carried.
    assert_eq!(state.nodes, 3); // NumNP, the 1772-1777 loop bound
    assert_eq!(state.mobile_residual_water, DefaultReal(0.1)); // Par(1,1)
    assert_eq!(state.mobile_saturated_water, DefaultReal(1.0)); // Par(2,1)
    assert_eq!(state.mobile_n, DefaultReal(2.0)); // Par(4,1)
    assert_eq!(state.mobile_tortuosity, DefaultReal(0.5)); // Par(6,1)
    assert_eq!(state.colloid_exclusion_water_content, DefaultReal(0.2)); // ChPar(4,1)
    assert_eq!(state.first_node_new_water, DefaultReal(0.6)); // ThNew(1)
    assert_eq!(state.first_node_new_velocity, DefaultReal(-0.25)); // vNew(1)
}

/// Write-only and dead actuals are not accepted as input: `ThOld`, `vOld`, the
/// non-first nodes of `ThNew`/`vNew`, and `Par(3,1)` have no field, and a request
/// naming one is rejected rather than silently ignored. `SinkS` is an output.
#[test]
fn coupling_bundles_reject_write_only_and_dead_state() {
    for key in [
        "old_water",
        "old_velocity",
        "mobile_alpha",
        "material_index",
        "species_dimension",
    ] {
        let mut request = coupling_request(control_section(false, false));
        insert(
            &mut request,
            "size_exclusion_state",
            &size_exclusion_bundle(&[(key, json(1.0))]),
        );
        let rejection = coupling_rejection(&request);
        assert!(rejection.contains("unknown nested key"), "{rejection}");
        assert!(rejection.contains("size_exclusion_state"), "{rejection}");
    }
    for key in ["solute_sink", "i_dual_por", "water_flow", "root_sink"] {
        let mut request = coupling_request(control_section(false, true));
        insert(
            &mut request,
            "root_uptake_state",
            &root_uptake_bundle(&[(key, json(1.0))]),
        );
        let rejection = coupling_rejection(&request);
        assert!(rejection.contains("unknown nested key"), "{rejection}");
    }
}

/// Every read actual is required: `rKM` and `cMin` are read only under
/// `lActRSU.and.NS.eq.1` (`INPUT.FOR:1123-1127`) and are never defaulted anywhere in
/// `src_reference`, so no reader default may stand in for a missing value.
#[test]
fn coupling_bundles_require_every_read_actual() {
    for key in [
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
    ] {
        let mut request = coupling_request(control_section(false, true));
        insert(
            &mut request,
            "root_uptake_state",
            &bundle_without(&root_uptake_entries(), key),
        );
        let rejection = coupling_rejection(&request);
        assert!(rejection.contains(key), "{key}: {rejection}");
    }
    for key in [
        "nodes",
        "mobile_residual_water",
        "mobile_saturated_water",
        "mobile_n",
        "mobile_tortuosity",
        "colloid_exclusion_water_content",
        "first_node_new_water",
        "first_node_new_velocity",
    ] {
        let mut request = coupling_request(control_section(false, false));
        insert(
            &mut request,
            "size_exclusion_state",
            &bundle_without(&size_exclusion_entries(), key),
        );
        let rejection = coupling_rejection(&request);
        assert!(rejection.contains(key), "{key}: {rejection}");
    }
}

/// The 38 pinned decks carry no root-uptake or size-exclusion state and enable
/// neither guard, so neither consumer is reachable from them.
/// The fourteen read actuals of the `SOLUTE.FOR:94-96` `SetSSnk` call.
fn root_uptake_entries() -> Vec<(&'static str, Value)> {
    vec![
        ("species", json(1)),
        ("species_count", json(1)),
        ("time", json(3600.0)),
        ("coordinates", array(&[json(0.0), json(-10.0), json(-20.0)])),
        (
            "root_distribution",
            array(&[json(1.0), json(1.0), json(1.0)]),
        ),
        (
            "root_water_sink",
            array(&[json(0.0), json(-10.0), json(-20.0)]),
        ),
        (
            "species_concentration",
            array(&[json(2.0), json(2.5), json(3.0)]),
        ),
        ("omega_water", json(0.8)),
        ("species_root_max_concentration", json(1.5)),
        ("lActRSU", json(false)),
        ("solute_stress_index", json(1.0)),
        ("potential_uptake", json(0.0)),
        ("michaelis_constant", json(1.0)),
        ("minimum_concentration", json(0.5)),
    ]
}

/// The seven values `SOLUTE.FOR:1713-1779` reads, plus `NumNP`, which the call
/// passes as the loop bound of `1772-1777`.
fn size_exclusion_entries() -> Vec<(&'static str, Value)> {
    vec![
        ("nodes", json(3)),
        ("mobile_residual_water", json(0.1)),
        ("mobile_saturated_water", json(1.0)),
        ("mobile_n", json(2.0)),
        ("mobile_tortuosity", json(0.5)),
        ("colloid_exclusion_water_content", json(0.2)),
        ("first_node_new_water", json(0.6)),
        ("first_node_new_velocity", json(-0.25)),
    ]
}

fn root_uptake_bundle(overrides: &[(&str, Value)]) -> Map<String, Value> {
    bundle(&root_uptake_entries(), overrides)
}

fn size_exclusion_bundle(overrides: &[(&str, Value)]) -> Map<String, Value> {
    bundle(&size_exclusion_entries(), overrides)
}

fn biology_mode_bundle() -> Map<String, Value> {
    bundle(&[("iBact", json(1)), ("lFiltr", json(false))], &[])
}

/// The `INPUT.FOR:31` control logicals `SinkF` and `lWat`.
fn control_section(l_wat: bool, sink_f: bool) -> Map<String, Value> {
    bundle(&[("lWat", json(l_wat)), ("SinkF", json(sink_f))], &[])
}

fn coupling_request(control: Map<String, Value>) -> Map<String, Value> {
    let mut request = Map::new();
    request.insert(
        "control_switches".into(),
        field("Site.dat control logicals", &control),
    );
    request
}

fn insert(request: &mut Map<String, Value>, key: &str, value: &Map<String, Value>) {
    let units = match key {
        "biology_mode" => "ChemIn source flags",
        "root_uptake_state" => "source root uptake state",
        "size_exclusion_state" => "source size-exclusion state",
        _ => "Site.dat control logicals",
    };
    request.insert(key.to_string(), field(units, value));
}

fn field(units: &str, value: &Map<String, Value>) -> Value {
    let mut out = Map::new();
    out.insert("units".to_string(), Value::String(units.to_string()));
    out.insert("value".to_string(), Value::Object(value.clone()));
    Value::Object(out)
}

fn bundle(entries: &[(&str, Value)], overrides: &[(&str, Value)]) -> Map<String, Value> {
    let mut object = Map::new();
    for (key, value) in entries.iter().chain(overrides.iter()) {
        object.insert((*key).to_string(), value.clone());
    }
    object
}

fn bundle_without(entries: &[(&str, Value)], key: &str) -> Map<String, Value> {
    let mut object = Map::new();
    for (k, value) in entries {
        if *k != key {
            object.insert((*k).to_string(), value.clone());
        }
    }
    object
}

fn array(entries: &[Value]) -> Value {
    Value::Array(entries.to_vec())
}

fn json(value: impl Into<Value>) -> Value {
    value.into()
}

/// Requests must carry the version the reader dispatches on (`adapter:157-159`).
fn with_schema(request: Map<String, Value>) -> Map<String, Value> {
    let mut envelope = Map::new();
    envelope.insert(
        "schema_version".to_string(),
        Value::String("solute-request-v1".into()),
    );
    envelope.insert("fields".to_string(), Value::Object(request));
    envelope
}

fn coupling_rejection(request: &Map<String, Value>) -> String {
    match parse_generated_request(request) {
        Ok(_) => panic!("expected a rejection"),
        Err(e) => e,
    }
}

/// Serializes a field map the way `CASE.json` does and parses it through the real
/// entry point, so every coupling test exercises the shipped validation path.
fn parse_generated_request(request: &Map<String, Value>) -> Result<ParsedRequest, String> {
    let text = Value::Object(with_schema(request.clone())).to_string();
    let hash = canonical_hash(&serde_json::from_str(&text).unwrap());
    parse_request(&format!(r#"{{"request":{text}}}"#), &hash)
}

fn parse_coupling_request(request: &Map<String, Value>) -> ParsedRequest {
    parse_generated_request(request)
        .unwrap_or_else(|e| panic!("expected acceptance, rejected: {e}"))
}

#[test]
fn pinned_requests_cannot_supply_the_source_coupling_state() {
    let root = Path::new("../../reference/fortran_reference/solute/parity");
    let root = if root.exists() {
        root.to_path_buf()
    } else {
        Path::new("reference/fortran_reference/solute/parity").to_path_buf()
    };
    let mut cases = 0;
    for entry in walkdir(&root) {
        let text = std::fs::read_to_string(&entry).unwrap();
        let parsed = parse_case_manifest(&text).unwrap();
        let coupling = &parsed.input.fields.source_coupling;
        assert!(coupling.control_switches.is_none(), "{}", entry.display());
        assert!(coupling.root_uptake_state.is_none());
        assert!(coupling.size_exclusion_state.is_none());
        // And no pinned deck enables either guard, so neither call could run.
        assert!(!parsed.input.switches.root_sink);
        cases += 1;
    }
    assert_eq!(cases, 44);
}

#[test]
fn all_wave3_case_requests_match_python_canonical_hash_and_map_to_typed_input() {
    let root = Path::new("../../reference/fortran_reference/solute/parity");
    let root = if root.exists() {
        root.to_path_buf()
    } else {
        Path::new("reference/fortran_reference/solute/parity").to_path_buf()
    };
    let mut count = 0;
    for entry in walkdir(&root) {
        let text = std::fs::read_to_string(&entry).unwrap();
        let parsed =
            parse_case_manifest(&text).unwrap_or_else(|e| panic!("{}: {e}", entry.display()));
        assert_eq!(parsed.input.request_sha256, parsed.request_sha256);
        assert!(parsed.input.validate_dimensions().is_ok());
        count += 1;
    }
    assert_eq!(count, 44);
}
fn walkdir(path: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(path).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(walkdir(&p));
        } else if p.file_name().is_some_and(|n| n == "CASE.json") {
            out.push(p)
        }
    }
    out
}
#[test]
fn rejects_duplicate_unknown_nonfinite_and_bad_hash_before_mapping() {
    assert!(parse_request(r#"{"request":{"request":{"schema_version":"solute-request-v1","fields":{"x":{"units":"1","value":1},"x":{"units":"1","value":2}}}}}"#, "a".repeat(64).as_str()).is_err());
    let request =
        r#"{"schema_version":"solute-request-v1","fields":{"mystery":{"units":"1","value":1}}}"#;
    let hash = canonical_hash(&serde_json::from_str(request).unwrap());
    assert!(parse_request(&format!(r#"{{"request":{request}}}"#), &hash)
        .unwrap_err()
        .contains("unmapped"));
    let request = r#"{"schema_version":"solute-request-v1","fields":{"temperature_history":{"units":"degC","value":NaN}}}"#;
    assert!(parse_request(&format!(r#"{{"request":{request}}}"#), &"0".repeat(64)).is_err());
}
#[test]
fn rejects_dimension_units_and_accepts_reader_reachable_nonlinear_mode() {
    let bad = r#"{"schema_version":"solute-request-v1","fields":{"linearity_mode":{"units":"boolean","value":false}}}"#;
    let hash = canonical_hash(&serde_json::from_str(bad).unwrap());
    let parsed = parse_request(&format!(r#"{{"request":{bad}}}"#), &hash).unwrap();
    assert!(!parsed.input.switches.linear);
    let bad = r#"{"schema_version":"solute-normalized-request-v1","fields":{"coordinates":{"units":"cm","value":{"native":"x","canonical":"d","nodes":2}},"temperature_history":{"units":"degC","value":[20.0]}}}"#;
    let hash = canonical_hash(&serde_json::from_str(bad).unwrap());
    assert!(parse_request(&format!(r#"{{"request":{bad}}}"#), &hash)
        .unwrap_err()
        .contains("temperature_history length"));
    let bad = r#"{"schema_version":"solute-request-v1","fields":{"time.interval":{"units":"second","value":1}}}"#;
    let hash = canonical_hash(&serde_json::from_str(bad).unwrap());
    assert!(parse_request(&format!(r#"{{"request":{bad}}}"#), &hash)
        .unwrap_err()
        .contains("expected units"));
}

#[test]
fn derives_source_selectors_and_primitive_endpoint_inputs() {
    let request = r#"{"schema_version":"solute-request-v1","fields":{"species_count":{"units":"count","value":1},"ordered_species":{"units":"ordered species records","value":[{"name":"S1","position":1,"chain_rate_per_hour":0.0,"initial_concentration_C_ref":1.0,"linear_equilibrium_kd_cm3_per_g":0.0}]},"biology_mode":{"units":"ChemIn source flags","value":{"iBact":0,"lFiltr":false}},"material_species_chpar":{"units":"source ChPar slots by species/material","value":[{"species":1,"material":1,"slots":[1.0,0.0,1.0,0.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]}]},"time.interval":{"units":"hour","value":1.0},"solute.flux_concentration":{"units":"boolean","value":true},"moisture_dependency":{"units":"source selector and WDep table","value":{"selector":1,"table":[1.0,0.0,0.8,0.2]}},"initialization.mass":{"units":"boolean","value":true},"initialization.equilibrium":{"units":"boolean","value":false},"initial_dissolved_concentration":{"units":"C_ref","value":[1.0]},"boundary.lower.code":{"units":"source integer code","value":1},"boundary.upper.code":{"units":"source integer code","value":1}}}"#;
    let parsed = parse_request_text(request);
    assert!(parsed.input.switches.linear);
    assert!(parsed.input.switches.flux_concentration);
    assert_eq!(parsed.input.switches.moisture_selector, 1);
    assert!(parsed.input.switches.mass_initialization);
    assert!(!parsed.input.switches.equilibrium_initialization);

    let mut nonlinear = serde_json::from_str::<Value>(request).unwrap();
    let slots = nonlinear["fields"]["material_species_chpar"]["value"][0]["slots"]
        .as_array_mut()
        .unwrap();
    slots[7] = Value::from(0.2);
    let hash = canonical_hash(&nonlinear);
    let wrapped = format!(r#"{{"request":{nonlinear},"request_sha256":"{hash}"}}"#);
    assert!(
        !parse_request(&wrapped, &hash)
            .unwrap()
            .input
            .switches
            .linear
    );
}

/// The smallest state both `MatSet` and the flux closure can consume: three
/// nodes, one material, both codes prescribed.
const ENDPOINT_BUNDLE: &str = r#""coordinates":[0.0,2.0,5.0],"old_water":[0.4,0.5,0.6],"new_water":[0.45,0.55,0.65],"old_velocity":[0.3,0.2,-0.4],"new_velocity":[0.35,0.25,-0.5],"old_matrix_velocity":[0.3,0.2,-0.4],"new_matrix_velocity":[0.35,0.25,-0.5],"old_dispersion":[0.1,0.2,0.3],"new_dispersion":[0.12,0.22,0.32],"old_retardation":[1.1,1.2,1.3],"new_retardation":[1.15,1.25,1.35],"old_concentration":[1.0,2.0,3.0],"new_accepted_concentration":[1.5,2.5,3.5],"material_immobile_water":[0.05],"source_g0":[0.01,0.02,0.03],"source_g1":[0.04,0.05,0.06],"upwind_weight":[0.2,0.3],"epsi":0.6,"dt":2.0,"alf":0.4,"material_ids":[1,1,1],"mobile_immobile":[false],"bottom_resistance":1.0,"bottom_prescribed":4.0,"top_prescribed":5.0,"t_level":2,"r_min":1e-30"#;

/// `ChPar(6,1)` and `ChPar(10,1)`, the gas-diffusion and Henry pair of `79`.
const ENDPOINT_CHPAR: &str = r#","source_chpar":[{"species":1,"material":1,"slots":[0.0,0.0,0.0,0.0,0.0,0.8,0.0,0.0,0.0,0.25,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]}]"#;

const TDEP_TEN: &str = r#","source_tdep":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]"#;

const ATMOSPHERE: &str = r#","old_temperature":15.0,"new_temperature":25.0,"top_surface_distance":3.0,"top_atmospheric_concentration":0.7"#;

/// Wraps the primitive bundle, plus any `","key":value` extras, in a request
/// whose `kTopCh` is `upper_code`.
fn endpoint_request(upper_code: i32, extra: &str) -> String {
    let primitives: Value = serde_json::from_str(&format!("{{{ENDPOINT_BUNDLE}{extra}}}")).unwrap();
    serde_json::json!({
        "schema_version": "solute-request-v1",
        "fields": {
            "boundary.lower.code": {"units": "source integer code", "value": 1},
            "boundary.upper.code": {"units": "source integer code", "value": upper_code},
            "endpoint_primitives": {
                "units": "source primitive endpoint state",
                "value": primitives,
            }
        }
    })
    .to_string()
}

fn endpoint_state(request: &str) -> EndpointPrimitiveState {
    let parsed = parse_request_text(request);
    let value = &parsed
        .input
        .fields
        .transport
        .endpoint_state
        .as_ref()
        .unwrap()
        .value;
    match value {
        Explicit::Available(state) => state.clone(),
        Explicit::Unavailable(_) => panic!("the endpoint bundle must be available"),
    }
}

fn endpoint_rejection(request: &str) -> String {
    let hash = canonical_hash(&serde_json::from_str(request).unwrap());
    parse_request(
        &format!(r#"{{"request":{request},"request_sha256":"{hash}"}}"#),
        &hash,
    )
    .unwrap_err()
}

#[test]
fn endpoint_primitives_are_exactly_the_state_the_source_reads() {
    let state = endpoint_state(&endpoint_request(1, &format!("{ENDPOINT_CHPAR}{TDEP_TEN}")));
    assert_eq!(state.source_species, 1);
    assert_eq!(state.t_level, 2);
    assert_eq!(state.material_ids, vec![1, 1, 1]);
    assert_eq!(state.source_tdep.as_ref().unwrap().len(), 10);
    let records = state.source_chpar.as_ref().expect("source ChPar records");
    // One record per species/material pair carries the 20 slots, because the
    // source indexes the material in the second ChPar dimension
    // (INPUT.FOR:1341-1352).
    assert_eq!(
        (
            records[0].species,
            records[0].material,
            records[0].slots.len()
        ),
        (1, 1, 20)
    );
    // Without kTopCh=-2 the atmospheric inputs stay optional (1409-1415).
    assert_eq!(state.old_temperature, None);
    assert_eq!(state.top_atmospheric_concentration, None);

    // ChPar(4,M) is one value per material (765, 771), so a three-node
    // bundle with two materials is the normal shape, not a length error.
    let two_materials = endpoint_state(&endpoint_request(
        1,
        r#","material_immobile_water":[0.05,0.0],"material_ids":[1,2,2],"mobile_immobile":[false,true]"#,
    ));
    assert_eq!(two_materials.material_immobile_water.len(), 2);
    assert_eq!(two_materials.material_ids, vec![1, 2, 2]);

    // kTopCh=-2 reads all six of them (74-84, 884-893, 917-925).
    let atmospheric = endpoint_state(&endpoint_request(
        -2,
        &format!("{ENDPOINT_CHPAR}{TDEP_TEN}{ATMOSPHERE}"),
    ));
    assert_eq!(atmospheric.old_temperature.unwrap().0, 15.0);
    assert_eq!(atmospheric.new_temperature.unwrap().0, 25.0);
    assert_eq!(
        atmospheric.top_surface_distance.unwrap().0,
        3.0,
        "dSurf is a primitive; dSurfT is not"
    );
    assert_eq!(atmospheric.top_atmospheric_concentration.unwrap().0, 0.7);
}

#[test]
fn endpoint_primitives_reject_derived_and_inconsistent_state() {
    // A precomputed Henry coefficient or gas diffusion would hide the
    // ChPar/TDep/TT derivation of 79-81 from the kernel.
    for key in [
        "henry_coefficient",
        "gas_diffusion",
        "old_accepted_flow",
        "new_accepted_flow",
        "old_accepted_concentration",
    ] {
        let rejection = endpoint_rejection(&endpoint_request(1, &format!(r#","{key}":0.1"#)));
        assert!(
            rejection.contains(&format!("unknown nested key {key}")),
            "{rejection}"
        );
    }

    // TDep is one-dimensional with a 16-entry species stride, and the
    // atmospheric code reads slots 6, 9 and 10 (INPUT.FOR:1379-1390).
    assert!(endpoint_rejection(&endpoint_request(
        1,
        r#","source_tdep":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]"#
    ))
    .contains("source_tdep"));
    assert!(endpoint_rejection(&endpoint_request(
        1,
        r#","source_chpar":[{"species":1,"material":4,"slots":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]}]"#
    ))
    .contains("source_chpar"));
    assert!(
        endpoint_rejection(&endpoint_request(1, r#","source_species":0"#))
            .contains("source_species")
    );
    assert!(
        endpoint_rejection(&endpoint_request(1, r#","material_ids":[1,1,3]"#))
            .contains("material_ids")
    );
    assert!(
        endpoint_rejection(&endpoint_request(1, r#","upwind_weight":[0.2]"#))
            .contains("upwind_weight")
    );
    // TLevel is only ever 1 or 2 when Solute calls MatSet (107, 124-128).
    assert!(endpoint_rejection(&endpoint_request(1, r#","t_level":0"#)).contains("TLevel"));
    assert!(endpoint_rejection(&endpoint_request(1, r#","t_level":3"#)).contains("TLevel"));

    // The atmospheric branch has no default for any of its six inputs.
    let missing = endpoint_rejection(&endpoint_request(
        -2,
        &format!("{ENDPOINT_CHPAR}{TDEP_TEN}"),
    ));
    assert!(
        missing.contains("atmospheric endpoint requires"),
        "{missing}"
    );
    // MatNum(N) (884, 917, 207) needs the ChPar record of this species at
    // the material of the top node, not just any record.
    let no_record = endpoint_rejection(&endpoint_request(
        -2,
        r#","mobile_immobile":[false,true],"material_ids":[2,2,2],"material_immobile_water":[0.05,0.0],"source_chpar":[{"species":1,"material":1,"slots":[0.0,0.0,0.0,0.0,0.0,0.8,0.0,0.0,0.0,0.25,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]}],"source_tdep":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],"old_temperature":15.0,"new_temperature":25.0,"top_surface_distance":3.0,"top_atmospheric_concentration":0.7"#,
    ));
    assert!(no_record.contains("source ChPar record"), "{no_record}");
}

#[test]
fn review_retry_contract_regressions() {
    let lower_type_three = parse_request_text(
        r#"{"schema_version":"solute-request-v1","fields":{"boundary.lower.code":{"units":"source integer code","value":-1}}}"#,
    );
    assert_eq!(
        lower_type_three.input.lower_boundary,
        Some(BoundaryForm::TypeThree)
    );
    let lower_zero = parse_request_text(
        r#"{"schema_version":"solute-request-v1","fields":{"boundary.lower.code":{"units":"source integer code","value":0}}}"#,
    );
    assert_eq!(
        lower_zero.input.lower_boundary,
        Some(BoundaryForm::ZeroGradient)
    );
    let upper_atmospheric = parse_request_text(
        r#"{"schema_version":"solute-request-v1","fields":{"boundary.upper.code":{"units":"source integer code","value":-2}}}"#,
    );
    assert_eq!(
        upper_atmospheric.input.upper_boundary,
        Some(BoundaryForm::Atmospheric)
    );
    let upper_type_three = parse_request_text(
        r#"{"schema_version":"solute-request-v1","fields":{"boundary.upper.code":{"units":"source integer code","value":0}}}"#,
    );
    assert_eq!(
        upper_type_three.input.upper_boundary,
        Some(BoundaryForm::TypeThree)
    );
    let upper_prescribed = parse_request_text(
        r#"{"schema_version":"solute-request-v1","fields":{"boundary.upper.code":{"units":"source integer code","value":1}}}"#,
    );
    assert_eq!(
        upper_prescribed.input.upper_boundary,
        Some(BoundaryForm::Prescribed)
    );
    let mobile_immobile = parse_request_text(
        r#"{"schema_version":"solute-request-v1","fields":{"equilibrium_mode":{"units":"boolean","value":false},"immobile_water_content":{"units":"cm3/cm3","value":0.1}}}"#,
    );
    assert!(mobile_immobile.input.switches.mobile_immobile);
    let bacteria = parse_request_text(
        r#"{"schema_version":"solute-request-v1","fields":{"equilibrium_mode":{"units":"boolean","value":false},"immobile_water_content":{"units":"cm3/cm3","value":0.1},"biology_mode":{"units":"ChemIn source flags","value":{"iBact":1,"lFiltr":false}}}}"#,
    );
    assert!(!bacteria.input.switches.mobile_immobile);
    assert!(bacteria.input.switches.bacteria);
    let dual = parse_request_text(
        r#"{"schema_version":"solute-request-v1","fields":{"equilibrium_mode":{"units":"boolean","value":false},"immobile_water_content":{"units":"cm3/cm3","value":0.1},"dual_domain":{"units":"source iDualPor selector","value":{"dynamic_exchange_claimed":false,"iDualPor":1,"water_model":6}}}}"#,
    );
    assert!(!dual.input.switches.mobile_immobile);
    assert_eq!(dual.input.switches.dual_domain_model, Some(6));
    let parsed = parse_request_text(
        r#"{"schema_version":"solute-normalized-request-v1","fields":{"coordinates":{"units":"cm","value":{"native":"x=-0..-250","canonical":"flow_distance=0..250","nodes":3}}}}"#,
    );
    let coords = parsed
        .input
        .fields
        .transport_properties
        .coordinates
        .unwrap();
    assert_eq!(
        coords.value,
        Explicit::Available(CoordinatesSpec {
            native: "x=-0..-250".into(),
            canonical: "flow_distance=0..250".into(),
            nodes: 3
        })
    );
    assert_eq!(
        parsed
            .input
            .fields
            .transport
            .flow_orientation
            .unwrap()
            .value,
        Explicit::Available(FlowOrientation::NegativeNative)
    );
    assert!(matches!(parsed.input.coordinates, Explicit::Unavailable(_)));
    assert_eq!(
        parsed.input.source_indices,
        Explicit::Available(SourceIndices {
            canonical_to_native: vec![1, 2, 3]
        })
    );
    let no_permutation = parse_request_text(
        r#"{"schema_version":"solute-request-v1","fields":{"linearity_mode":{"units":"boolean","value":true}}}"#,
    );
    assert!(matches!(
        no_permutation.input.source_indices,
        Explicit::Unavailable(_)
    ));
    let lower_unsupported = r#"{"schema_version":"solute-request-v1","fields":{"boundary.lower.code":{"units":"source integer code","value":-2}}}"#;
    let hash = canonical_hash(&serde_json::from_str(lower_unsupported).unwrap());
    assert!(
        parse_request(&format!(r#"{{"request":{lower_unsupported}}}"#), &hash)
            .unwrap_err()
            .contains("unsupported lower boundary")
    );
    let mixed_record = r#"{"schema_version":"solute-request-v1","fields":{"linearity_mode":{"units":"boolean","value":true,"availability":"available","reason":"mixed"}}}"#;
    let hash = canonical_hash(&serde_json::from_str(mixed_record).unwrap());
    assert!(
        parse_request(&format!(r#"{{"request":{mixed_record}}}"#), &hash)
            .unwrap_err()
            .contains("mutually exclusive")
    );
    let unknown_nested = r#"{"schema_version":"solute-normalized-request-v1","fields":{"coordinates":{"units":"cm","value":{"native":"x=-0..-250","canonical":"flow_distance=0..250","nodes":3,"x":[0,1,2]}}}}"#;
    let hash = canonical_hash(&serde_json::from_str(unknown_nested).unwrap());
    assert!(
        parse_request(&format!(r#"{{"request":{unknown_nested}}}"#), &hash)
            .unwrap_err()
            .contains("unknown nested key")
    );
    let bad_species = r#"{"schema_version":"solute-request-v1","fields":{"species_count":{"units":"count","value":2},"ordered_species":{"units":"ordered species records","value":[{"name":"S1","position":1,"chain_rate_per_hour":0.0,"initial_concentration_C_ref":1.0,"linear_equilibrium_kd_cm3_per_g":0.0}]}}}"#;
    let hash = canonical_hash(&serde_json::from_str(bad_species).unwrap());
    assert!(
        parse_request(&format!(r#"{{"request":{bad_species}}}"#), &hash)
            .unwrap_err()
            .contains("species_count")
    );

    let bad_nodes = r#"{"schema_version":"solute-normalized-request-v1","fields":{"coordinates":{"units":"cm","value":{"native":"x=-0..-250","canonical":"flow_distance=0..250","nodes":3}},"initial_dual_immobile_water_content":{"units":"cm3/cm3","value":[0.1,0.2]}}}"#;
    let hash = canonical_hash(&serde_json::from_str(bad_nodes).unwrap());
    assert!(
        parse_request(&format!(r#"{{"request":{bad_nodes}}}"#), &hash)
            .unwrap_err()
            .contains("initial_dual_immobile_water_content length")
    );

    let request = r#"{"schema_version":"solute-request-v1","fields":{"linearity_mode":{"units":"boolean","value":true}}}"#;
    let hash = canonical_hash(&serde_json::from_str(request).unwrap());
    let manifest_without_inputs = format!(r#"{{"request":{request},"request_sha256":"{hash}"}}"#);
    assert!(parse_case_manifest(&manifest_without_inputs)
        .unwrap_err()
        .contains("input_files"));
    let manifest_bad_hash = format!(
        r#"{{"request":{request},"request_sha256":"{hash}","input_files":{{"SELECTOR.IN":"not-a-hash"}}}}"#
    );
    assert!(parse_case_manifest(&manifest_bad_hash)
        .unwrap_err()
        .contains("input_files.SELECTOR.IN"));
}

/// A model-6 deck: three nodes on 0.25 cm spacing, one material carrying the
/// `dual_model6` `ParD` column (`mobile_residual_water`/`mobile_saturated_water`
/// are `ParD(1)`/`ParD(2)`, `mobile_tortuosity` is `ParD(6)`, and the
/// immobile triple is `ParD(7)`/`ParD(8)`/`ParD(9)`), and the reader's own
/// `hCritA` (`INPUT.FOR:102-103`).
const DUAL_POROSITY_BUNDLE: &str = r#""coordinates":[0.0,0.25,0.5],"old_water":[0.3,0.3,0.3],"new_pressure_head":[-100.0,-100.0,-100.0],"old_immobile_water":[0.1,0.1,0.1],"material_ids":[1,1,1],"materials":[{"mobile_residual_water":0.08,"mobile_saturated_water":0.43,"mobile_tortuosity":0.5,"immobile_residual_water":0.05,"immobile_saturated_water":0.25,"immobile_alpha":0.1}],"critical_atmospheric_head":-1e10,"dt":0.1,"hydraulic_model":0,"dual_porosity":1"#;

fn dual_porosity_bundle() -> Value {
    serde_json::from_str(&format!("{{{DUAL_POROSITY_BUNDLE}}}")).unwrap()
}

fn dual_porosity_request(bundle: Value, selector: Value) -> String {
    serde_json::json!({
        "schema_version": "solute-request-v1",
        "fields": {
            "dual_domain": {"units": "source iDualPor selector", "value": selector},
            "dual_porosity_exchange_state": {
                "units": "source primitive dual-porosity exchange state",
                "value": bundle,
            },
        }
    })
    .to_string()
}

fn dual_porosity_state(bundle: Value, selector: Value) -> DualPorPrimitiveState {
    let parsed = parse_request_text(&dual_porosity_request(bundle, selector));
    let value = &parsed
        .input
        .fields
        .nonequilibrium
        .dual_porosity_exchange_state
        .as_ref()
        .unwrap()
        .value;
    match value {
        Explicit::Available(state) => state.clone(),
        Explicit::Unavailable(_) => panic!("the exchange bundle must be available"),
    }
}

fn dual_porosity_rejection(request: &str) -> String {
    let hash = canonical_hash(&serde_json::from_str(request).unwrap());
    parse_request(
        &format!(r#"{{"request":{request},"request_sha256":"{hash}"}}"#),
        &hash,
    )
    .unwrap_err()
}

/// `"available"`, `"unavailable"` or `"absent"` for one optional field.
fn explicit_state<T>(field: Option<&Explicit<T>>) -> String {
    match field {
        None => "absent".to_string(),
        Some(Explicit::Available(_)) => "available".to_string(),
        Some(Explicit::Unavailable(_)) => "unavailable".to_string(),
    }
}

#[test]
fn dual_porosity_exchange_state_maps_the_state_the_source_reads() {
    let state = dual_porosity_state(
        dual_porosity_bundle(),
        serde_json::json!({ "iDualPor": 1, "water_model": 6 }),
    );
    assert_eq!(state.coordinates.len(), 3);
    assert_eq!(state.old_water, vec![DefaultReal(0.3); 3]);
    assert_eq!(state.new_pressure_head, vec![DefaultReal(-100.0); 3]);
    assert_eq!(state.old_immobile_water, vec![DefaultReal(0.1); 3]);
    assert_eq!(state.material_ids, vec![1, 1, 1]);
    assert_eq!(state.critical_atmospheric_head, DefaultReal(-1e10));
    assert_eq!(state.dt, DefaultReal(0.1));
    assert_eq!((state.hydraulic_model, state.dual_porosity), (0, 1));
    let material = &state.materials[0];
    assert_eq!(material.mobile_residual_water, DefaultReal(0.08));
    assert_eq!(material.mobile_saturated_water, DefaultReal(0.43));
    assert_eq!(material.mobile_tortuosity, Some(DefaultReal(0.5)));
    assert_eq!(material.immobile_residual_water, DefaultReal(0.05));
    assert_eq!(material.immobile_saturated_water, DefaultReal(0.25));
    assert_eq!(material.immobile_alpha, DefaultReal(0.1));
    // A model-6 deck fills ParD(1..9,M) only (INPUT.FOR:618, NPar=9), so the
    // ParD(10)/ParD(11) slots a model 7 deck adds stay absent.
    assert_eq!(material.immobile_n, None);
    assert_eq!(material.immobile_saturated_conductivity, None);
}

#[test]
fn dual_porosity_exchange_state_rejects_unknown_missing_and_ragged_state() {
    let selector = serde_json::json!({ "iDualPor": 1, "water_model": 6 });

    // thr and ths have no key of their own: GenMat derives them from
    // ParD(1,M) and ParD(2,M) (INPUT.FOR:796-797), so the mobile pair is the
    // only spelling and a second one would be able to contradict it.
    let mut derived_pair = dual_porosity_bundle();
    derived_pair["materials"][0]["thr"] = serde_json::json!(0.08);
    assert!(
        dual_porosity_rejection(&dual_porosity_request(derived_pair, selector.clone()))
            .contains("unknown nested key thr")
    );

    // SinkIm and WTransf are results, and ThNewIm reaches ThOldIm only
    // through Update (WATFLOW.FOR:1112), so no output key belongs here.
    for output in ["sink_immobile", "transfer", "new_immobile_water"] {
        let mut bundle = dual_porosity_bundle();
        bundle[output] = serde_json::json!([0.0, 0.0, 0.0]);
        assert!(
            dual_porosity_rejection(&dual_porosity_request(bundle, selector.clone()))
                .contains(&format!("unknown nested key {output}"))
        );
    }

    // hCritA is required and never defaulted.
    let mut without_h_crit_a = dual_porosity_bundle();
    without_h_crit_a
        .as_object_mut()
        .unwrap()
        .remove("critical_atmospheric_head");
    assert!(
        dual_porosity_rejection(&dual_porosity_request(without_h_crit_a, selector.clone()))
            .contains("critical_atmospheric_head missing")
    );

    // ParD(9,M) is read by both dual models (WATFLOW.FOR:1037, 1044).
    let mut without_alpha = dual_porosity_bundle();
    without_alpha.as_object_mut().unwrap()["materials"][0]
        .as_object_mut()
        .unwrap()
        .remove("immobile_alpha");
    assert!(
        dual_porosity_rejection(&dual_porosity_request(without_alpha, selector.clone()))
            .contains("immobile_alpha missing")
    );

    // Mixed-length quantities share one wrapper, so the units are fixed.
    let mut wrong_units = serde_json::from_str::<Value>(&dual_porosity_request(
        dual_porosity_bundle(),
        selector.clone(),
    ))
    .unwrap();
    wrong_units["fields"]["dual_porosity_exchange_state"]["units"] = serde_json::json!("cm3/cm3");
    assert!(dual_porosity_rejection(&wrong_units.to_string()).contains("expected units"));

    // The four node arrays are the node count of the mesh's own coordinate
    // description (x, ThOld, hNew and ThOldIm in WATFLOW.FOR:1029-1031).
    let mut ragged = dual_porosity_bundle();
    ragged["old_water"] = serde_json::json!([0.3, 0.3]);
    let mut ragged_request =
        serde_json::from_str::<Value>(&dual_porosity_request(ragged, selector.clone())).unwrap();
    ragged_request["fields"]["coordinates"] = serde_json::json!({
        "units": "cm",
        "value": {"native": "x=0..0.5", "canonical": "flow_distance=0..0.5", "nodes": 3},
    });
    assert!(
        dual_porosity_rejection(&ragged_request.to_string()).contains(
            "dual_porosity_exchange_state: dual-porosity exchange node arrays must all be the same length"
        )
    );

    let mut empty_materials = dual_porosity_bundle();
    empty_materials["materials"] = serde_json::json!([]);
    assert!(
        dual_porosity_rejection(&dual_porosity_request(empty_materials, selector.clone()))
            .contains(
            "dual_porosity_exchange_state: dual-porosity exchange needs at least one ParD column"
        )
    );

    for id in [0, 2] {
        let mut bad_ids = dual_porosity_bundle();
        bad_ids["material_ids"] = serde_json::json!([1, 1, id]);
        assert!(
            dual_porosity_rejection(&dual_porosity_request(bad_ids, selector.clone()))
                .contains("dual_porosity_exchange_state: MatNum must index the ParD columns")
        );
    }

    let mut text_slot = dual_porosity_bundle();
    text_slot["materials"][0]["immobile_alpha"] = serde_json::json!("0.1");
    assert!(
        dual_porosity_rejection(&dual_porosity_request(text_slot, selector))
            .contains("expected finite number")
    );
}

#[test]
fn dual_porosity_exchange_state_agrees_with_the_dual_domain_selector() {
    // iDualPor reaches the bundle and the guard of WATFLOW.FOR:43 from one
    // deck, so the two spellings may not diverge.
    let mismatch = dual_porosity_rejection(&dual_porosity_request(
        dual_porosity_bundle(),
        serde_json::json!({ "iDualPor": 2, "water_model": 7, "dynamic_exchange_claimed": true }),
    ));
    assert!(mismatch.contains("must match dual_domain.iDualPor"));

    // INPUT.FOR:619-624 replaces models 6 and 7 with iModel=0 before ParD is
    // read, so a bundle naming the deck model as iModel is not a state the
    // call can be given.
    let unremapped = dual_porosity_rejection(
        &dual_porosity_request(
            dual_porosity_bundle(),
            serde_json::json!({ "iDualPor": 1, "water_model": 6 }),
        )
        .replace("\"hydraulic_model\":0", "\"hydraulic_model\":6"),
    );
    assert!(unremapped.contains("reach DualPor as iModel 0"));

    // A model 7 deck fills ParD(6), ParD(10) and ParD(11) (INPUT.FOR:623);
    // the model-6 bundle above cannot serve it.
    let mut model7_bundle = dual_porosity_bundle();
    model7_bundle["dual_porosity"] = serde_json::json!(2);
    let missing_model7_slots = dual_porosity_rejection(&dual_porosity_request(
        model7_bundle.clone(),
        serde_json::json!({ "iDualPor": 2, "water_model": 7 }),
    ));
    assert!(missing_model7_slots.contains("water model 7 fills ParD(6,M), ParD(10,M)"));

    model7_bundle["materials"][0]["immobile_n"] = serde_json::json!(1.2);
    model7_bundle["materials"][0]["immobile_saturated_conductivity"] = serde_json::json!(0.01);
    let state = dual_porosity_state(
        model7_bundle,
        serde_json::json!({ "iDualPor": 2, "water_model": 7 }),
    );
    assert_eq!(state.dual_porosity, 2);
    assert_eq!(state.materials[0].immobile_n, Some(DefaultReal(1.2)));
    assert_eq!(
        state.materials[0].immobile_saturated_conductivity,
        Some(DefaultReal(0.01))
    );
}

/// Input blocker, recorded as a test: the pinned dual-domain requests carry
/// the `iDualPor` selector but none of the node state `DualPor` reads, so no
/// `dual_porosity_exchange_state` can be built from them without synthesizing
/// values (see the 18.7.3 report).
#[test]
fn pinned_requests_cannot_supply_the_dual_porosity_exchange_state() {
    let root = Path::new("../../reference/fortran_reference/solute/parity");
    let root = if root.exists() {
        root.to_path_buf()
    } else {
        Path::new("reference/fortran_reference/solute/parity").to_path_buf()
    };
    let mut dual_decks = 0;
    for entry in walkdir(&root) {
        let text = std::fs::read_to_string(&entry).unwrap();
        let parsed = parse_case_manifest(&text).unwrap();
        let fields = &parsed.input.fields;
        assert!(fields.nonequilibrium.dual_porosity_exchange_state.is_none());
        if !entry.to_string_lossy().contains("nonequilibrium/dual") {
            continue;
        }
        dual_decks += 1;
        let selector = match &fields.nonequilibrium.dual_domain.as_ref().unwrap().value {
            Explicit::Available(selector) => selector.clone(),
            Explicit::Unavailable(_) => panic!("the dual decks carry a dual_domain selector"),
        };
        // The selector side of the state exists in every dual deck, and the
        // reader sets iDualPor above zero only for water models 6 and 7
        // (INPUT.FOR:619-624).
        assert_eq!(
            selector.i_dual_por > 0,
            matches!(selector.water_model, 6 | 7),
            "{entry:?}: {}",
            selector.i_dual_por
        );
        // None of the node state does: x is a range description, and ThOld,
        // hNew and ThOldIm are left to the run the case does not replay.
        assert_ne!(
            explicit_state(
                fields
                    .transport_properties
                    .coordinates
                    .as_ref()
                    .map(|v| &v.value)
            ),
            "available"
        );
        assert_eq!(
            explicit_state(
                fields
                    .nonequilibrium
                    .initial_water_content
                    .as_ref()
                    .map(|v| &v.value)
            ),
            "unavailable"
        );
        if selector.i_dual_por > 0 {
            assert_eq!(
                explicit_state(
                    fields
                        .nonequilibrium
                        .initial_dual_immobile_water_content
                        .as_ref()
                        .map(|v| &v.value)
                ),
                "unavailable"
            );
        }
        // The decks also claim SinkIm is unreachable because
        // "WATFLOW.FOR:42-46 comments out call DualPor". That claim is not
        // what the pinned source says: 43 is an `if(iDualPor.gt.0)` and only
        // the continuation lines 44-46 are commented, i.e. the call is live
        // (see the 18.7.3 notes). A corrected fixture must trip this
        // assertion together with a corrected request.
        let sink = fields
            .nonequilibrium
            .dynamic_exchange_sink
            .as_ref()
            .unwrap()
            .value
            .unavailable_reason()
            .unwrap()
            .to_string();
        assert!(sink.contains("comments out call DualPor"), "{sink}");
    }
    assert_eq!(dual_decks, 3);
}

/// Request literals taken from the rejection tests above; the snapshot records the
/// parse outcome of each so later serde work can diff the error text.
const SNAPSHOT_REJECTION_REQUESTS: &[&str] = &[
    r#"{"schema_version":"solute-request-v1","fields":{"x":{"units":"1","value":1},"x":{"units":"1","value":2}}}"#,
    r#"{"schema_version":"solute-request-v1","fields":{"mystery":{"units":"1","value":1}}}"#,
    r#"{"schema_version":"solute-request-v1","fields":{"temperature_history":{"units":"degC","value":NaN}}}"#,
    r#"{"schema_version":"solute-normalized-request-v1","fields":{"coordinates":{"units":"cm","value":{"native":"x","canonical":"d","nodes":2}},"temperature_history":{"units":"degC","value":[20.0]}}}"#,
    r#"{"schema_version":"solute-request-v1","fields":{"time.interval":{"units":"second","value":1}}}"#,
];

/// Writes the parsed fixtures and the rejection texts to `HYDRUS_ADAPTER_SNAPSHOT`.
/// Run with `--ignored`; the file is line oriented so two snapshots can be diffed.
#[test]
#[ignore]
fn dump_parsed_fixtures() {
    let target = std::env::var("HYDRUS_ADAPTER_SNAPSHOT")
        .expect("set HYDRUS_ADAPTER_SNAPSHOT to the snapshot output path");
    let root = Path::new("../../reference/fortran_reference/solute/parity");
    let root = if root.exists() {
        root.to_path_buf()
    } else {
        Path::new("reference/fortran_reference/solute/parity").to_path_buf()
    };
    let mut cases = walkdir(&root);
    cases.sort();
    let mut out = String::from("## parsed_fixtures\n");
    for case in &cases {
        let text = std::fs::read_to_string(case).unwrap();
        let name = case.strip_prefix(&root).unwrap().display();
        out.push_str(&format!(
            "fixture\t{name}\t{:?}\n",
            parse_case_manifest(&text)
        ));
    }
    out.push_str("## rejection_texts\n");
    for (index, request) in SNAPSHOT_REJECTION_REQUESTS.iter().enumerate() {
        let hash = match serde_json::from_str::<Value>(request) {
            Ok(value) => canonical_hash(&value),
            Err(_) => "a".repeat(64),
        };
        let result = parse_request(&format!(r#"{{"request":{request}}}"#), &hash);
        out.push_str(&format!("rejection\t{index}\t{result:?}\n"));
    }
    std::fs::write(target, out).unwrap();
}

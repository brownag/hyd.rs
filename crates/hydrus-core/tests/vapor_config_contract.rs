use hydrus_core::heat::config::{HeatConfigError, HeatConfigInput};
use hydrus_core::heat::driver::{HeatDriverError, HeatWaterDriver};
use hydrus_core::heat::transport::{solve_heat_transport, HeatTransportError, HeatTransportInput};
use hydrus_core::heat::vapor::{VaporConfig, VaporConfigInput, VaporError, VaporPrimitiveInput};
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, Mesh1D,
};
use hydrus_core::{
    HydraulicError, LowerBoundaryMode, UpperBoundary, WaterState, WaterStepInput, WaterStepPolicy,
    WaterWorkspace,
};
use hydrus_core::{RootUptakeInput, RootUptakeModel};

fn heat() -> HeatConfigInput {
    HeatConfigInput {
        l_vapor: true,
        l_en_bal: false,
        i_dual_por: 0,
        l_w_dep: false,
        i_model: 0,
        domain_count: 1,
        k_top_t: 1,
        t_period: 0.0,
        k_bot_t: 1,
        campbell: false,
    }
}
fn input() -> VaporConfigInput {
    VaporConfigInput {
        heat: heat(),
        water_enabled: true,
        heat_enabled: true,
        i_enhanc: 1,
        hysteresis: 0,
        chemistry: false,
        density_coupling: false,
        centrifugal: false,
        top_temperature: 20.0,
        bottom_temperature: 20.0,
        x_conv: 1.0,
        t_conv: 1.0,
    }
}
fn config() -> VaporConfig {
    VaporConfig::new(input()).unwrap()
}

#[test]
fn accepts_only_the_supported_configuration() {
    let c = config();
    assert_eq!(c.enhancement(), 1);
    assert_eq!(c.hydraulic_model(), 0);
    assert_eq!(c.top_temperature(), 20.0);
}

#[test]
fn rejects_each_excluded_heat_and_vapor_selector() {
    let mut cases = Vec::new();
    let mut h = heat();
    h.campbell = true;
    cases.push(VaporConfigInput { heat: h, ..input() });
    let mut h = heat();
    h.k_top_t = -2;
    cases.push(VaporConfigInput { heat: h, ..input() });
    let mut h = heat();
    h.l_en_bal = true;
    cases.push(VaporConfigInput { heat: h, ..input() });
    let mut h = heat();
    h.i_dual_por = 1;
    cases.push(VaporConfigInput { heat: h, ..input() });
    let mut h = heat();
    h.l_w_dep = true;
    cases.push(VaporConfigInput { heat: h, ..input() });
    let mut h = heat();
    h.i_model = 1;
    cases.push(VaporConfigInput { heat: h, ..input() });
    let mut h = heat();
    h.domain_count = 2;
    cases.push(VaporConfigInput { heat: h, ..input() });
    let mut h = heat();
    h.k_top_t = 0;
    cases.push(VaporConfigInput { heat: h, ..input() });
    for period in [1.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut h = heat();
        h.t_period = period;
        cases.push(VaporConfigInput { heat: h, ..input() });
    }
    for bad in [
        VaporConfigInput {
            water_enabled: false,
            ..input()
        },
        VaporConfigInput {
            heat_enabled: false,
            ..input()
        },
        VaporConfigInput {
            i_enhanc: 0,
            ..input()
        },
        VaporConfigInput {
            hysteresis: 1,
            ..input()
        },
        VaporConfigInput {
            chemistry: true,
            ..input()
        },
        VaporConfigInput {
            density_coupling: true,
            ..input()
        },
        VaporConfigInput {
            centrifugal: true,
            ..input()
        },
    ] {
        cases.push(bad);
    }
    for case in cases {
        assert!(
            VaporConfig::new(case).is_err(),
            "accepted excluded selector: {case:?}"
        );
    }
}

#[test]
fn rejects_nonfinite_and_invalid_temperatures_and_conversions() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(VaporConfig::new(VaporConfigInput {
            top_temperature: value,
            ..input()
        })
        .is_err());
        assert!(VaporConfig::new(VaporConfigInput {
            bottom_temperature: value,
            ..input()
        })
        .is_err());
        assert!(VaporConfig::new(VaporConfigInput {
            x_conv: value,
            ..input()
        })
        .is_err());
        assert!(VaporConfig::new(VaporConfigInput {
            t_conv: value,
            ..input()
        })
        .is_err());
    }
    assert!(matches!(
        VaporConfig::new(VaporConfigInput {
            top_temperature: -273.15,
            ..input()
        }),
        Err(VaporError::InvalidValue { .. })
    ));
    assert!(VaporConfig::new(VaporConfigInput {
        x_conv: 0.0,
        ..input()
    })
    .is_err());
    assert!(VaporConfig::new(VaporConfigInput {
        t_conv: -1.0,
        ..input()
    })
    .is_err());
}

struct Model;
impl HydraulicModel for Model {
    fn evaluate(&self, _: f64) -> Result<Evaluation, HydraulicError> {
        unreachable!()
    }
    fn derivatives(&self, _: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        unreachable!()
    }
}
static MODEL: Model = Model;
static MATERIALS: [&dyn HydraulicModel; 1] = [&MODEL];
fn row() -> BoundaryRow {
    BoundaryRow {
        lower: 0.0,
        diagonal: 0.0,
        upper: 0.0,
        rhs: 0.0,
    }
}
fn water_input<'a>(
    mesh: &'a Mesh1D,
    sink: &'a [f64],
    lower: LowerBoundaryMode,
    upper: UpperBoundary,
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials: &MATERIALS,
        sink,
        root_uptake: None,
        timestep: 1.0,
        target_time: 1.0,
        face_averaging: FaceAveraging::Arithmetic,
        lower,
        upper,
    }
}
fn boundaries() -> (LowerBoundaryMode, UpperBoundary) {
    (
        LowerBoundaryMode::FixedHead {
            head: 0.0,
            row: row(),
            flux: 0.0,
        },
        UpperBoundary::Head {
            head: 0.0,
            row: row(),
            flux: 0.0,
        },
    )
}
fn policy() -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations: 10,
        head_tolerance: 1e-6,
        water_tolerance: 1e-6,
        minimum_timestep: 1e-6,
        retry_factor: 3.0,
        minimum_pivot: 1e-12,
    }
}

#[test]
fn preflight_rejects_bad_dimensions_mapping_modes_and_sink() {
    let c = config();
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0, 0, 0], None, 1, 0, 0.0).unwrap();
    let (lower, upper) = boundaries();
    assert!(c
        .validate_water_input(
            &water_input(&mesh, &[0.0, 0.0], lower, upper),
            0.0,
            policy()
        )
        .is_err());
    assert!(matches!(
        c.validate_water_input(
            &water_input(&mesh, &[0.0, 1.0, 0.0], lower, upper),
            0.0,
            policy()
        ),
        Err(VaporError::NonzeroSink { .. })
    ));
    let bad_mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0, 1, 0], None, 2, 0, 0.0).unwrap();
    assert!(matches!(
        c.validate_water_input(
            &water_input(&bad_mesh, &[0.0; 3], lower, upper),
            0.0,
            policy()
        ),
        Err(VaporError::LengthMismatch { .. })
    ));
    let error = c
        .validate_water_input(
            &water_input(
                &mesh,
                &[0.0; 3],
                LowerBoundaryMode::PrescribedFlux { flux: 0.0 },
                upper,
            ),
            0.0,
            policy(),
        )
        .unwrap_err();
    assert!(
        matches!(error, VaporError::UnsupportedLowerBoundary(_)),
        "{error:?}"
    );
    assert!(matches!(
        c.validate_water_input(
            &water_input(
                &mesh,
                &[0.0; 3],
                lower,
                UpperBoundary::Flux {
                    flux: 0.0,
                    row: row()
                }
            ),
            0.0,
            policy()
        ),
        Err(VaporError::UnsupportedUpperBoundary(_))
    ));
    let mut bad = water_input(&mesh, &[0.0; 3], lower, upper);
    bad.face_averaging = FaceAveraging::Geometric;
    assert!(matches!(
        c.validate_water_input(&bad, 0.0, policy()),
        Err(VaporError::UnsupportedAveraging)
    ));
    let roots = [1.0; 3];
    let root = RootUptakeInput {
        model: RootUptakeModel::Feddes,
        h0: 0.0,
        h1: -1.0,
        h2: -2.0,
        h3: -3.0,
        potential_demand: 1.0,
        compensation_threshold: 1.0,
        root_weights: &roots,
    };
    let mut bad = water_input(&mesh, &[0.0; 3], lower, upper);
    bad.root_uptake = Some(&root);
    assert!(matches!(
        c.validate_water_input(&bad, 0.0, policy()),
        Err(VaporError::RootUptake)
    ));
}

#[test]
fn primitive_arrays_are_required_and_validated() {
    let empty: &[f64] = &[];
    let mut p = VaporPrimitiveInput {
        heads: &[0.0; 3],
        temperatures: &[20.0; 3],
        liquid_water_contents: &[0.2; 3],
        liquid_conductivities: &[1.0; 3],
        liquid_capacities: &[1.0; 3],
        material_indices: &[0; 3],
        saturated_water_contents: &[0.4],
        x_conv: 1.0,
        t_conv: 1.0,
    };
    assert!(p.validate().is_ok());
    p.temperatures = empty;
    assert!(matches!(
        p.validate(),
        Err(VaporError::LengthMismatch {
            name: "temperatures",
            ..
        })
    ));
}

#[test]
fn legacy_entries_reject_before_mutation() {
    let state = WaterState::new(0.0, vec![0.0; 3], vec![0.2; 3]).unwrap();
    let workspace = WaterWorkspace::new(3).unwrap();
    let before_state = state.clone();
    let before_workspace = workspace.clone();
    let result = HeatWaterDriver::new(heat(), &[], &[], &[], 0.0, vec![], &[]);
    assert!(matches!(
        result,
        Err(HeatDriverError::Config(
            HeatConfigError::VaporNotSupported { value: true }
        ))
    ));
    assert_eq!(state, before_state);
    assert_eq!(format!("{workspace:?}"), format!("{before_workspace:?}"));

    let heat_config = hydrus_core::heat::config::HeatConfig::new(heat()).unwrap();
    let empty = [];
    let empty_indices: [usize; 0] = [];
    let input = HeatTransportInput {
        x: &empty,
        dt: 1.0,
        material_indices: &empty_indices,
        materials: &[],
        temp_old: &empty,
        water_old: &empty,
        water_new: &empty,
        velocity_old: &empty,
        velocity_new: &empty,
        sink: &empty,
        top_temperature: 20.0,
        bottom_temperature: 20.0,
        x_conv: 1.0,
        t_conv: 1.0,
        minimum_pivot: 1e-12,
    };
    let mut output = [987.0; 3];
    assert!(matches!(
        solve_heat_transport(heat_config, input, &mut output),
        Err(HeatTransportError::VaporNotSupported)
    ));
    assert_eq!(output, [987.0; 3]);
}

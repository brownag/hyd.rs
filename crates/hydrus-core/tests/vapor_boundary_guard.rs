use hydrus_core::heat::config::HeatConfigInput;
use hydrus_core::heat::vapor::{VaporConfig, VaporConfigInput, VaporError};
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, Mesh1D,
};
use hydrus_core::{
    HydraulicError, LowerBoundaryMode, UnsupportedLowerBoundaryMode, UpperBoundary, WaterStepInput,
    WaterStepPolicy,
};

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

fn all_lowers() -> Vec<LowerBoundaryMode> {
    vec![
        LowerBoundaryMode::FixedHead {
            head: 0.0,
            row: row(),
            flux: 0.0,
        },
        LowerBoundaryMode::PrescribedFlux { flux: 0.5 },
        LowerBoundaryMode::FreeDrainage,
        LowerBoundaryMode::Unsupported(UnsupportedLowerBoundaryMode::SeepageFace),
        LowerBoundaryMode::Unsupported(UnsupportedLowerBoundaryMode::WaterTable),
    ]
}
fn all_uppers(flux_row: BoundaryRow) -> Vec<UpperBoundary> {
    vec![
        UpperBoundary::Head {
            head: 0.0,
            row: row(),
            flux: 0.0,
        },
        UpperBoundary::Flux {
            flux: -0.25,
            row: flux_row,
        },
        UpperBoundary::Atmospheric {
            critical_head: -1.0,
        },
    ]
}

#[test]
fn every_non_pair_combination_keeps_its_unsupported_error() {
    let c = config();
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0, 0, 0], None, 1, 0, 0.0).unwrap();
    for (i, lower) in all_lowers().into_iter().enumerate() {
        for (j, upper) in all_uppers(row()).into_iter().enumerate() {
            let got =
                c.validate_water_input(&water_input(&mesh, &[0.0; 3], lower, upper), 0.0, policy());
            match (i, j) {
                (0, 0) | (1, 1) => assert_eq!(got, Ok(()), "{lower:?} {upper:?}"),
                (0, _) => assert_eq!(got, Err(VaporError::UnsupportedUpperBoundary(upper))),
                _ => assert_eq!(got, Err(VaporError::UnsupportedLowerBoundary(lower))),
            }
        }
    }
}

#[test]
fn flux_row_is_ignored_on_the_vapor_path() {
    let c = config();
    let mesh = Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0, 0, 0], None, 1, 0, 0.0).unwrap();
    let lower = LowerBoundaryMode::PrescribedFlux { flux: 0.5 };
    let wild = BoundaryRow {
        lower: 7.0,
        diagonal: f64::NAN,
        upper: -3.0,
        rhs: f64::INFINITY,
    };
    for r in [row(), wild] {
        let upper = UpperBoundary::Flux {
            flux: -0.25,
            row: r,
        };
        assert_eq!(
            c.validate_water_input(&water_input(&mesh, &[0.0; 3], lower, upper), 0.0, policy()),
            Ok(())
        );
    }
    let bad = UpperBoundary::Flux {
        flux: f64::NAN,
        row: row(),
    };
    assert!(c
        .validate_water_input(&water_input(&mesh, &[0.0; 3], lower, bad), 0.0, policy())
        .is_err());
}

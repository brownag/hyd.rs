//! Per-step interior water and heat balance of the vapor driver for the accepted heat-boundary
//! forms DF, DZ, FF and FZ.
//!
//! Setup (assumptions): the seeded synthetic 9-node mesh and state from `tests/vapor_balance.rs`
//! (not the case inputs in `reference/vapor_fortran/cases/`). Only the heat boundary codes vary,
//! taken from the SELECTOR.IN "boundary code record" line:
//!   DF `(kTopT, kBotT) = (1, -1)`, DZ `(1, 0)`, FF `(-1, -1)`, FZ `(-1, 0)`.
//! Water boundaries are Dirichlet heads at both ends, as in SELECTOR.IN. FD is not included.
//! Bounds are read directly from `reference/vapor_fortran/criteria.json` (`checks`), with a
//! schema and scale check. Only interior rows are asserted. Fresh-coefficient closure is
//! printed as a diagnostic and never asserted.

use hydrus_core::heat::capacity::TempMaterial;
use hydrus_core::heat::config::HeatConfigInput;
use hydrus_core::heat::driver::CoupledHeatInput;
use hydrus_core::heat::vapor::balance::{vapor_balance, VaporBalanceInput};
use hydrus_core::heat::vapor::driver::{VaporStepReport, VaporWaterHeatDriver};
use hydrus_core::heat::vapor::kernels;
use hydrus_core::heat::vapor::state::VaporStateInput;
use hydrus_core::heat::vapor::{
    VaporConfig, VaporConfigInput, VaporPrimitiveInput, VaporSourceMaterial,
};
use hydrus_core::hydraulics::{BoundaryRow, FaceAveraging, Mesh1D};
use hydrus_core::{
    HydraulicModel, LowerBoundaryMode, UpperBoundary, VanGenuchtenModel, VanGenuchtenParameters,
    WaterState, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

/// Acceptance bound pair for one balance check.
struct Bound {
    relative: f64,
    absolute: f64,
}

/// Reads `checks.<name>` from `reference/vapor_fortran/criteria.json`.
fn load_bound(name: &str) -> Bound {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../reference/vapor_fortran/criteria.json"
    );
    let bytes = std::fs::read(path).expect("criteria.json readable");
    let doc: serde_json::Value = serde_json::from_slice(&bytes).expect("criteria.json parses");
    assert_eq!(doc["schema_version"], "vapor-criteria-v2", "schema_version");
    let c = &doc["checks"][name];
    assert_eq!(c["scale"], "max_abs_row_term", "{name} scale");
    let num = |key: &str| {
        let v = c[key]
            .as_f64()
            .unwrap_or_else(|| panic!("{name}.{key} not a number"));
        assert!(v.is_finite() && v >= 0.0, "{name}.{key} invalid: {v}");
        v
    };
    Bound {
        relative: num("relative"),
        absolute: num("absolute"),
    }
}

/// Heat boundary codes for one accepted form.
#[derive(Clone, Copy, Debug)]
struct Form {
    name: &'static str,
    k_top_t: i32,
    k_bot_t: i32,
}

const FORMS: [Form; 4] = [
    Form {
        name: "DF",
        k_top_t: 1,
        k_bot_t: -1,
    },
    Form {
        name: "DZ",
        k_top_t: 1,
        k_bot_t: 0,
    },
    Form {
        name: "FF",
        k_top_t: -1,
        k_bot_t: -1,
    },
    Form {
        name: "FZ",
        k_top_t: -1,
        k_bot_t: 0,
    },
];

const STEPS: usize = 4;
const XC: f64 = 100.0;
/// `INPUT.FOR:186-197`: `tConv = 1/(60*60)` for hours.
const TC: f64 = 1.0 / 3600.0;
const SEED: u64 = 0x5eed_2562;

/// Tiny deterministic generator (SplitMix64) so the mesh is seeded and reproducible.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn config(form: Form) -> VaporConfig {
    VaporConfig::new(VaporConfigInput {
        heat: HeatConfigInput {
            l_vapor: true,
            l_en_bal: false,
            i_dual_por: 0,
            l_w_dep: false,
            i_model: 0,
            domain_count: 1,
            k_top_t: form.k_top_t,
            t_period: 0.0,
            k_bot_t: form.k_bot_t,
            campbell: false,
        },
        water_enabled: true,
        heat_enabled: true,
        i_enhanc: 1,
        hysteresis: 0,
        chemistry: false,
        density_coupling: false,
        centrifugal: false,
        top_temperature: 35.0,
        bottom_temperature: 10.0,
        x_conv: XC,
        t_conv: TC,
    })
    .unwrap()
}

fn row(value: f64) -> BoundaryRow {
    BoundaryRow {
        lower: 0.0,
        diagonal: 1.0,
        upper: 0.0,
        rhs: value,
    }
}

fn policy() -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations: 50,
        head_tolerance: 1.0e-6,
        water_tolerance: 1.0e-9,
        minimum_timestep: 1.0e-8,
        retry_factor: 3.0,
        minimum_pivot: 1.0e-14,
    }
}

/// Report plus the pre-step temperatures, liquid and vapor velocities.
type StepRecord = (VaporStepReport, Vec<f64>, Vec<f64>, Vec<f64>);

struct Run {
    reports: Vec<StepRecord>,
    x: Vec<f64>,
    materials: Vec<TempMaterial>,
    sink: Vec<f64>,
    model: VanGenuchtenModel,
}

fn run(form: Form, steps: usize, seed: u64) -> Run {
    let mut rng = Rng(seed);
    let n = 9;
    let mut x = vec![0.0];
    for _ in 1..n {
        let last = *x.last().unwrap();
        x.push(last + 6.0 + 14.0 * rng.next());
    }
    let model =
        VanGenuchtenModel::new(VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 0.5).unwrap())
            .unwrap();
    let mesh = Mesh1D::new(x.clone(), vec![0; n], None, 1, 1, 1.0).unwrap();
    let top = x[n - 1];
    let heads: Vec<f64> = (0..n)
        .map(|i| -2000.0 - 4000.0 * x[i] / top - 200.0 * rng.next())
        .collect();
    let temps: Vec<f64> = (0..n).map(|i| 10.0 + 25.0 * x[i] / top).collect();
    let evals: Vec<_> = heads.iter().map(|&h| model.evaluate(h).unwrap()).collect();
    let theta: Vec<f64> = evals.iter().map(|e| e.theta).collect();
    let cond: Vec<f64> = evals.iter().map(|e| e.conductivity).collect();
    let cap: Vec<f64> = evals.iter().map(|e| e.capacity).collect();
    let materials = {
        // SI inputs converted to model units: capacity / (xConv*tConv^2), conductivity *
        // xConv/tConv^3 (same factors as the source's `Cv` and Campbell unit factor).
        let kc = 1.0 / (XC * TC * TC);
        let kl = XC / (TC * TC * TC);
        vec![TempMaterial::from_tpar([
            0.5,
            0.0,
            0.0,
            0.25 * kl,
            0.5 * kl,
            1.0 * kl,
            1.92e6 * kc,
            0.0,
            4.18e6 * kc,
        ])]
    };
    let mut driver = VaporWaterHeatDriver::new(
        config(form),
        VaporStateInput {
            primitive: VaporPrimitiveInput {
                heads: &heads,
                temperatures: &temps,
                liquid_water_contents: &theta,
                liquid_conductivities: &cond,
                liquid_capacities: &cap,
                material_indices: &vec![0; n],
                saturated_water_contents: &[0.45],
                x_conv: XC,
                t_conv: TC,
            },
            coordinates: &x,
            gravity: 1.0,
            initial_head_predictor: &heads,
            initialization_timestep: 0.05,
            time: 0.0,
        },
        &vec![0; n],
        &materials,
    )
    .unwrap();
    let mut state = WaterState::new(0.0, heads.clone(), theta).unwrap();
    let mut workspace = WaterWorkspace::new(n).unwrap();
    let sink = vec![0.0; n];
    let bm = [&model as &dyn HydraulicModel];
    let source = [VaporSourceMaterial {
        model: &model,
        table: None,
    }];
    let mut reports = Vec::new();
    for k in 1..=steps {
        let snap = driver.state().snapshot();
        let old = (
            snap.temperatures.to_vec(),
            snap.liquid_velocities.to_vec(),
            snap.vapor_velocities.to_vec(),
        );
        let input = WaterStepInput {
            mesh: &mesh,
            materials: &bm,
            sink: &sink,
            root_uptake: None,
            timestep: 0.05,
            target_time: 0.05 * k as f64,
            face_averaging: FaceAveraging::Arithmetic,
            lower: LowerBoundaryMode::FixedHead {
                head: heads[0],
                row: row(heads[0]),
                flux: 0.0,
            },
            upper: UpperBoundary::Head {
                head: heads[n - 1],
                row: row(heads[n - 1]),
                flux: 0.0,
            },
        };
        let report = driver
            .step(
                &mut state,
                &mut workspace,
                input,
                policy(),
                &source,
                CoupledHeatInput {
                    top_temperature: 35.0,
                    bottom_temperature: 10.0,
                    x_conv: XC,
                    t_conv: TC,
                    minimum_pivot: 1.0e-14,
                },
            )
            .unwrap();
        reports.push((report, old.0, old.1, old.2));
    }
    Run {
        reports,
        x,
        materials,
        sink,
        model,
    }
}

fn input<'a>(r: &'a Run, old: &'a (Vec<f64>, Vec<f64>, Vec<f64>)) -> VaporBalanceInput<'a> {
    VaporBalanceInput {
        coordinates: &r.x,
        gravity: 1.0,
        x_conv: XC,
        t_conv: TC,
        sink: &r.sink,
        old_temperatures: &old.0,
        old_liquid_velocities: &old.1,
        old_vapor_velocities: &old.2,
        heat_materials: &r.materials,
        material_indices: &[0; 9],
        campbell: false,
    }
}

#[test]
fn accepted_forms_meet_approved_interior_balance_bounds() {
    let water_bound = load_bound("water_balance_interior");
    let heat_bound = load_bound("heat_balance_interior");
    for form in FORMS {
        let run = run(form, STEPS, SEED);
        let (mut worst_w, mut worst_h) = (0.0_f64, 0.0_f64);
        for (k, (report, o_t, o_v, o_vv)) in run.reports.iter().enumerate() {
            let old = (o_t.clone(), o_v.clone(), o_vv.clone());
            let b = vapor_balance(&input(&run, &old), report).unwrap();
            // Non-vacuous: vapor storage and vapor transport are nonzero.
            assert!(
                b.water.vapor_storage.iter().any(|v| v.abs() > 0.0),
                "{} step {k}: vapor storage is zero",
                form.name
            );
            assert!(
                b.water.vapor_head_transport.iter().any(|v| v.abs() > 0.0),
                "{} step {k}: vapor head transport is zero",
                form.name
            );
            assert!(
                b.heat.vapor_latent.iter().any(|v| v.abs() > 0.0),
                "{} step {k}: vapor latent term is zero",
                form.name
            );
            let wr = b
                .water
                .worst_ratio(water_bound.relative, water_bound.absolute);
            let hr = b.heat.worst_ratio(heat_bound.relative, heat_bound.absolute);
            worst_w = worst_w.max(wr);
            worst_h = worst_h.max(hr);
            println!(
                "{} step {k}: water worst ratio {wr:.3e}, heat worst ratio {hr:.3e}",
                form.name
            );
            assert!(
                wr <= 1.0,
                "{} step {k}: water balance ratio {wr} > 1",
                form.name
            );
            assert!(
                hr <= 1.0,
                "{} step {k}: heat balance ratio {hr} > 1",
                form.name
            );
            for v in b.water.residual.iter().chain(&b.heat.residual) {
                assert!(v.is_finite(), "{} step {k}: non-finite residual", form.name);
            }
        }
        println!(
            "{}: max water worst ratio {worst_w:.3e}, max heat worst ratio {worst_h:.3e} (bounds rel {:.1e}, abs {:.1e})",
            form.name, water_bound.relative, water_bound.absolute
        );
    }
}

#[test]
fn fresh_coefficient_closure_is_reported_not_asserted() {
    for form in FORMS {
        let run = run(form, 2, SEED);
        for (k, (r, o_t, o_v, o_vv)) in run.reports.iter().enumerate() {
            let old = (o_t.clone(), o_v.clone(), o_vv.clone());
            let b = vapor_balance(&input(&run, &old), r).unwrap();
            let h = &r.passes[1].heads;
            let evals: Vec<_> = h.iter().map(|&v| run.model.evaluate(v).unwrap()).collect();
            let theta: Vec<f64> = evals.iter().map(|e| e.theta).collect();
            let cond: Vec<f64> = evals.iter().map(|e| e.conductivity).collect();
            let cap: Vec<f64> = evals.iter().map(|e| e.capacity).collect();
            let fresh = kernels::evaluate(&VaporPrimitiveInput {
                heads: h,
                temperatures: &r.passes[1].temperature_estimate,
                liquid_water_contents: &theta,
                liquid_conductivities: &cond,
                liquid_capacities: &cap,
                material_indices: &[0; 9],
                saturated_water_contents: &[0.45],
                x_conv: XC,
                t_conv: TC,
            })
            .unwrap();
            let mut worst = 0.0_f64;
            #[allow(clippy::needless_range_loop)]
            for i in 1..8 {
                let dx = (run.x[i + 1] - run.x[i - 1]) / 2.0;
                let storage = dx
                    * ((theta[i] - r.liquid_contents_old[i])
                        + (fresh.vapor_contents[i] - r.vapor_contents_old[i]))
                    / r.timestep;
                let res = storage
                    + b.water.liquid_transport[i - 1]
                    + b.water.vapor_head_transport[i - 1]
                    + b.water.vapor_thermal_transport[i - 1];
                worst = worst.max(res.abs() / b.water.scale[i - 1]);
            }
            println!(
                "{} step {k}: fresh-coefficient continuum closure max rel {worst:.3e} (diagnostic only)",
                form.name
            );
            assert!(worst.is_finite());
        }
    }
}

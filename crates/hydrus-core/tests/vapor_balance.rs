//! Interior water and heat balance of the vapor driver over several accepted steps.
//!
//! Setup (assumptions): seeded nonuniform 9-node mesh (cm), dry state (heads about -2e3..-6e3 cm),
//! linear temperature profile with Dirichlet ends, cm/hour units (`x_conv=100`, `t_conv=3600`),
//! enhancement on, no sink, Dirichlet heads both ends. Only interior rows are asserted.
//! Bounds come from `reference/vapor_fortran/criteria.json`.
//! Dirichlet reservoir bookkeeping and closure against freshly re-evaluated coefficients are
//! printed as diagnostics and never asserted.

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
use sha2::Digest;

/// Acceptance bounds loaded from the canonical `reference/vapor_fortran/criteria.json`.
struct Bound {
    relative: f64,
    absolute: f64,
}

fn load_bounds() -> (Bound, Bound) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../reference/vapor_fortran/criteria.json"
    );
    let bytes = std::fs::read(path).expect("criteria.json readable");
    println!("criteria.json sha256 {:x}", sha2::Sha256::digest(&bytes));
    let doc: serde_json::Value = serde_json::from_slice(&bytes).expect("criteria.json parses");
    assert_eq!(doc["schema_version"], "vapor-criteria-v2", "schema_version");
    let checks = doc["checks"].as_object().expect("checks object");
    let get = |name: &str| {
        let c = checks
            .get(name)
            .unwrap_or_else(|| panic!("missing check {name}"));
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
    };
    (get("water_balance_interior"), get("heat_balance_interior"))
}

/// Minimum vapor/liquid ratio asserted for storage and transport, chosen before measuring:
/// at a dry state vapor is physically small but must not be negligible beyond 1e-6.
const MIN_VAPOR_LIQUID_RATIO: f64 = 1.0e-6;

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

fn config() -> VaporConfig {
    VaporConfig::new(VaporConfigInput {
        heat: HeatConfigInput {
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

fn run(steps: usize, seed: u64) -> Run {
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
        config(),
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
fn interior_water_and_heat_residuals_meet_approved_bounds() {
    let (water_bound, heat_bound) = load_bounds();
    let run = run(4, SEED);
    let (mut max_w, mut max_h) = (0.0_f64, 0.0_f64);
    for (k, (report, o_t, o_v, o_vv)) in run.reports.iter().enumerate() {
        let old = (o_t.clone(), o_v.clone(), o_vv.clone());
        let b = vapor_balance(&input(&run, &old), report).unwrap();
        // Non-vacuous: vapor storage and every vapor transport term are nonzero.
        assert!(
            b.water.vapor_storage.iter().any(|v| v.abs() > 0.0),
            "step {k}"
        );
        assert!(b.water.vapor_head_transport.iter().any(|v| v.abs() > 0.0));
        assert!(b
            .water
            .vapor_thermal_transport
            .iter()
            .any(|v| v.abs() > 0.0));
        assert!(b.heat.vapor_sensible.iter().any(|v| v.abs() > 0.0));
        assert!(b.heat.vapor_latent.iter().any(|v| v.abs() > 0.0));
        let mx = |v: &[f64]| v.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
        let vap_tr: Vec<f64> = b
            .water
            .vapor_head_transport
            .iter()
            .zip(&b.water.vapor_thermal_transport)
            .map(|(a, c)| a + c)
            .collect();
        let r_tr = mx(&vap_tr) / mx(&b.water.liquid_transport);
        let r_st = mx(&b.water.vapor_storage) / mx(&b.water.liquid_storage);
        println!("  vapor/liquid ratios: transport {r_tr:.3e}, storage {r_st:.3e}");
        assert!(
            r_tr >= MIN_VAPOR_LIQUID_RATIO && r_st >= MIN_VAPOR_LIQUID_RATIO,
            "step {k}"
        );
        let wr = b
            .water
            .worst_ratio(water_bound.relative, water_bound.absolute);
        let hr = b.heat.worst_ratio(heat_bound.relative, heat_bound.absolute);
        max_w = max_w.max(b.water.max_relative());
        max_h = max_h.max(b.heat.max_relative());
        println!(
            "step {k}: water max rel {:.3e} (published-flux form max abs {:.3e}, flux diff {:.3e}), heat max rel {:.3e}",
            b.water.max_relative(),
            b.water.residual_published_fluxes.iter().fold(0.0_f64, |m, v| m.max(v.abs())),
            b.water.published_flux_difference,
            b.heat.max_relative()
        );
        println!(
            "  diag: corrected-content residual max {:.3e}, storage-form diff max {:.3e}, source storage max {:.3e}",
            b.water.corrected_content_residual.iter().fold(0.0_f64, |m, v| m.max(v.abs())),
            b.water.storage_form_difference.iter().fold(0.0_f64, |m, v| m.max(v.abs())),
            b.water.source_storage.iter().fold(0.0_f64, |m, v| m.max(v.abs())),
        );
        println!("  reservoir bookkeeping (NOT closure): {:?}", b.reservoir);
        assert!(wr <= 1.0, "step {k}: water ratio {wr}");
        assert!(hr <= 1.0, "step {k}: heat ratio {hr}");
        // Finite and signed diagnostics.
        for v in b.water.residual.iter().chain(&b.heat.residual) {
            assert!(v.is_finite());
        }
        for (i, &c) in report.vapor_head_conductivities.iter().enumerate() {
            assert!(c >= 0.0 && report.vapor_contents_new[i] >= 0.0, "node {i}");
        }
        // Isothermal-head sign: vapor head flux is opposite to the head gradient.
        let h = &report.passes[1].heads;
        for i in 1..h.len() {
            let dh = h[i] - h[i - 1];
            let qv_head = -(report.vapor_head_conductivities[i]
                + report.vapor_head_conductivities[i - 1])
                / 2.0
                * dh
                / (run.x[i] - run.x[i - 1]);
            assert!(qv_head * dh <= 0.0);
        }
        for i in 0..report.total_face_fluxes.len() {
            let sum = report.liquid_face_fluxes[i] + report.vapor_face_fluxes[i];
            assert_eq!(report.total_face_fluxes[i], sum);
        }
        // Published flux recomputation (same arithmetic, roundoff bound 1e-12 relative to the largest flux).
        let big = report
            .total_face_fluxes
            .iter()
            .fold(0.0_f64, |m, v| m.max(v.abs()));
        assert!(
            b.water.published_flux_difference <= 1.0e-12 * big,
            "step {k}"
        );
    }
    println!("max rel water {max_w:.3e} heat {max_h:.3e}");
}

#[test]
fn fresh_coefficient_closure_is_reported_not_asserted() {
    let run = run(2, SEED);
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
            "step {k}: fresh-coefficient continuum closure max rel {worst:.3e} (reported only)"
        );
        assert!(worst.is_finite());
    }
}

#[test]
fn rejects_mismatched_lengths() {
    let run = run(1, SEED);
    let (report, o_t, o_v, o_vv) = &run.reports[0];
    let old = (o_t.clone(), o_v.clone(), o_vv.clone());
    let mut bad = input(&run, &old);
    let short = vec![0.0; 3];
    bad.sink = &short;
    assert!(vapor_balance(&bad, report).is_err());
    let mut r2 = report.clone();
    r2.vapor_contents_new[2] = f64::NAN;
    assert!(vapor_balance(&input(&run, &old), &r2).is_err());
}

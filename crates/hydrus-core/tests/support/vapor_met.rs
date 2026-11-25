//! Reusable met-run driver for the vapor + Meteo.in case family (`vapor_met_bare_soil_on`).
//!
//! Included by tests with `#[path = "support/vapor_met.rs"] mod vapor_met;`. The run path is
//! `src/`: `MetForcing` turns the `Meteo.in` and `ATMOSPH.IN` records into the forcing schedule
//! and top temperatures, and `VaporWaterHeatDriver::step_atmospheric` runs each step (two
//! source-ordered water/heat passes and the commit). This file is wiring: it parses the committed
//! deck (`SELECTOR.IN`, `PROFILE.DAT`, `ATMOSPH.IN`, `Meteo.in`), applies `RunOptions`, walks the
//! k/10 time grid, samples print times and builds the `StepTrace` records from the step report.
#![allow(dead_code)]

use std::{collections::BTreeMap, fs, path::PathBuf};

use hydrus_core::atmospheric::{VaporAtmosphericState, VaporSwitchRecord};
use hydrus_core::forcing::VaporAtmosphericCoupling;
use hydrus_core::heat::capacity::TempMaterial;
use hydrus_core::heat::config::HeatConfigInput;
use hydrus_core::heat::driver::CoupledHeatInput;
use hydrus_core::heat::vapor::driver::VaporWaterHeatDriver;
use hydrus_core::heat::vapor::kernels;
use hydrus_core::heat::vapor::output::{
    observe_step, observe_vapor, t_level_fluxes_for_step, VaporObservationInput,
};
use hydrus_core::heat::vapor::state::VaporStateInput;
use hydrus_core::heat::vapor::{
    VaporConfig, VaporConfigInput, VaporPrimitiveInput, VaporSourceMaterial,
};
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, HydraulicTables,
    Mesh1D, UnitSymbol, VanGenuchtenModel, VanGenuchtenParameters,
};
use hydrus_core::meteo::parse_meteo_deck;
use hydrus_core::{
    AtmosphericRates, AtmosphericRecord, ForcingUnits, HydraulicError, LowerBoundaryMode,
    MetForcing, UpperBoundary, WaterState, WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

pub const X_CONV: f64 = 100.0;
pub const T_CONV: f64 = 1.0 / 3600.0;
pub const MINIMUM_PIVOT: f64 = 1e-12;
pub const CASE: &str = "vapor_met_bare_soil_on";
pub const VAPOR_FIELDS: [&str; 9] = [
    "con",
    "con_lt",
    "con_vh",
    "con_vt",
    "v_liquid",
    "v_vapor",
    "v_total",
    "v_vap_iso",
    "v_vap_term",
];

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

pub fn case_dir() -> PathBuf {
    repo_root().join("reference/vapor_fortran/cases").join(CASE)
}

/// Run controls for diagnosis; `Default` is the full pinned run.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunOptions {
    /// Stop after this many accepted steps.
    pub max_steps: Option<usize>,
    /// Probe: start with a linear profile to this top-node head, in head mode (`KodTop = 4`, `hTop` = value).
    pub top_head_override: Option<f64>,
}

/// One accepted step. `time` is the accepted time at the end of the step.
#[derive(Clone, Debug)]
pub struct StepTrace {
    pub index: usize,
    pub time: f64,
    pub timestep: f64,
    pub kod_top: i8,
    pub h_top: f64,
    pub r_top: f64,
    pub v_top: f64,
    pub top_head: f64,
    pub iterations: [usize; 2],
    /// Switches in pass one and pass two, in decision order (pass number first).
    pub switches: Vec<(u8, VaporSwitchRecord)>,
    pub below_critical_head: [bool; 2],
    /// Pass-two (committed trial) face fluxes, node 0 = bottom face, `n` = top face, `n + 1` entries.
    pub liquid_face_fluxes: Vec<f64>,
    pub vapor_face_fluxes: Vec<f64>,
    pub total_face_fluxes: Vec<f64>,
    /// Bottom-first node contents before and after the step (cm3/cm3).
    pub liquid_contents_old: Vec<f64>,
    pub liquid_contents_new: Vec<f64>,
    pub vapor_contents_old: Vec<f64>,
    pub vapor_contents_new: Vec<f64>,
    /// Reported `TLInf` boundary fluxes (`OUTPUT.FOR:25-54`), positive upward (cm/h).
    pub v_top_reported: f64,
    pub v_bot_reported: f64,
    pub v_top_liquid_reported: f64,
    pub v_top_vapor_reported: f64,
}

/// A change of `KodTop` between consecutive accepted steps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KodChange {
    pub time: f64,
    pub from: i8,
    pub to: i8,
}

pub struct MetRun {
    pub time_points: Vec<f64>,
    pub node_depths: Vec<f64>,
    /// Top-first, time-major, like the fixture.
    pub head: Vec<Vec<f64>>,
    pub liquid_moisture: Vec<Vec<f64>>,
    pub temperature_c: Vec<Vec<f64>>,
    pub vapor: BTreeMap<&'static str, Vec<Vec<f64>>>,
    pub steps: Vec<StepTrace>,
    /// `(tAtm, rates)` per record, rates in cm/h, as given to the forcing schedule.
    pub records: Vec<(f64, AtmosphericRates)>,
    pub dt: f64,
}

impl MetRun {
    /// `KodTop` changes between accepted steps (initial value `-4`).
    pub fn kod_changes(&self) -> Vec<KodChange> {
        let mut out = Vec::new();
        let mut kod = -4;
        for s in &self.steps {
            if s.kod_top != kod {
                out.push(KodChange {
                    time: s.time,
                    from: kod,
                    to: s.kod_top,
                });
                kod = s.kod_top;
            }
        }
        out
    }
}

struct SpanModel<'a> {
    table: &'a HydraulicTables,
    analytic: &'a VanGenuchtenModel,
}

impl SpanModel<'_> {
    fn inside(&self, head: f64) -> bool {
        head >= self.table.heads()[0] && head <= *self.table.heads().last().unwrap()
    }
}

impl HydraulicModel for SpanModel<'_> {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        if self.inside(head) {
            self.table.evaluate(head)
        } else {
            self.analytic.evaluate(head)
        }
    }
    fn derivatives(&self, head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        if self.inside(head) {
            self.table.derivatives(head)
        } else {
            self.analytic.derivatives(head)
        }
    }
}

fn row(rhs: f64) -> BoundaryRow {
    BoundaryRow {
        lower: 0.0,
        diagonal: 1.0,
        upper: 0.0,
        rhs,
    }
}

fn nums(line: &str) -> Vec<f64> {
    line.split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect()
}

/// The line after the first line containing `key`.
fn after<'a>(lines: &[&'a str], key: &str) -> &'a str {
    let i = lines
        .iter()
        .position(|l| l.contains(key))
        .unwrap_or_else(|| panic!("deck line {key:?} missing"));
    lines[i + 1]
}

fn rev(values: &[f64]) -> Vec<f64> {
    values.iter().rev().copied().collect()
}

fn err<E: std::fmt::Debug>(context: &str) -> impl Fn(E) -> String + '_ {
    move |e| format!("{context}: {e:?}")
}

fn vapor_rows(
    obs: &hydrus_core::heat::vapor::output::VaporObservation,
    n: usize,
) -> BTreeMap<&'static str, Vec<f64>> {
    let mut out: BTreeMap<&'static str, Vec<f64>> = VAPOR_FIELDS
        .iter()
        .map(|&f| (f, vec![f64::NAN; n]))
        .collect();
    for r in &obs.rows {
        let top_first = n - 1 - r.rust_index;
        for (name, value) in [
            ("con", r.con),
            ("con_lt", r.con_lt),
            ("con_vh", r.con_vh),
            ("con_vt", r.con_vt),
            ("v_liquid", r.v_liquid),
            ("v_vapor", r.v_vapor),
            ("v_total", r.v_total),
            ("v_vap_iso", r.v_vap_iso),
            ("v_vap_term", r.v_vap_term),
        ] {
            out.get_mut(name).unwrap()[top_first] = value;
        }
    }
    out
}

/// Runs the committed deck. Errors are strings (deck, forcing or solver failures).
pub fn run_met_case(options: RunOptions) -> Result<MetRun, String> {
    let dir = case_dir();
    let rd = |name: &str| fs::read_to_string(dir.join(name)).map_err(|e| format!("{name}: {e}"));
    let selector = rd("SELECTOR.IN")?;
    let profile_text = rd("PROFILE.DAT")?;
    let atm_text = rd("ATMOSPH.IN")?;
    let meteo_text = rd("Meteo.in")?;
    let sel: Vec<&str> = selector.lines().collect();

    // Deck facts that must hold for this driver (nothing is assumed silently).
    let flags: Vec<&str> = after(&sel, "lWat lChem lTemp").split_whitespace().collect();
    let flags2: Vec<&str> = after(&sel, "lSnow lHP1").split_whitespace().collect();
    assert_eq!(
        (flags[0], flags[1], flags[2], flags[8]),
        ("t", "f", "t", "t")
    );
    assert_eq!((flags2[2], flags2[3]), ("t", "t"), "lMeteo and lVapor on");
    let cos_alpha = nums(after(&sel, "NMat NLay CosAlpha"))[2];
    let tols = nums(after(&sel, "MaxIt TolTh TolH"));
    let top: Vec<&str> = after(&sel, "TopInf WLayer KodTop")
        .split_whitespace()
        .collect();
    assert_eq!((top[0], top[1], top[2]), ("t", "f", "-4"), "TopInf, KodTop");
    let bot: Vec<&str> = after(&sel, "BotInf qGWLF").split_whitespace().collect();
    assert_eq!(
        (bot[0], bot[2], bot[4]),
        ("f", "f", "1"),
        "fixed-head bottom"
    );
    let htab = nums(after(&sel, "hTab1 hTabN"));
    let vg = nums(after(&sel, "thr ths Alfa n Ks l"));
    let time = nums(after(&sel, "dt dtMin dtMax"));
    let tmax = nums(after(&sel, "tInit tMax"));
    let tprint = nums(after(&sel, "TPrint(1...MPL)"));
    let tpar: [f64; 9] = nums(after(&sel, "record 2 (TempIn skip)"))
        .try_into()
        .map_err(|_| "TPar row".to_string())?;
    let bc = nums(after(&sel, "boundary code record"));
    assert_eq!((bc[0], bc[2]), (1.0, 1.0), "Dirichlet temperature ends");
    let (dt, dt_min) = (time[0], time[1]);
    let mut time_points = vec![tmax[0]];
    time_points.extend(&tprint);

    // PROFILE.DAT (top-first).
    let (mut depths, mut heads_tf, mut temps_tf) = (Vec::new(), Vec::new(), Vec::new());
    for line in profile_text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 10 && f[0].parse::<usize>().is_ok() {
            depths.push(f[1].parse::<f64>().unwrap());
            heads_tf.push(f[2].parse::<f64>().unwrap());
            temps_tf.push(f[9].parse::<f64>().unwrap());
        }
    }
    let n = depths.len();
    assert_eq!(n, 41);
    let x = rev(&depths);
    let mut heads = rev(&heads_tf);
    let temps = rev(&temps_tf);
    let indices = vec![0usize; n];
    if let Some(h) = options.top_head_override {
        // Probe profile: linear from the bottom head to `h` at the surface.
        let bottom = heads[0];
        for (i, v) in heads.iter_mut().enumerate() {
            *v = bottom + (h - bottom) * i as f64 / (n - 1) as f64;
        }
    }

    // ATMOSPH.IN: tAtm Prec rSoil rRoot hCritA rB hB ht tTop tBot Ampl (11 columns).
    let atm_lines: Vec<&str> = atm_text.lines().collect();
    let first = atm_lines
        .iter()
        .position(|l| l.trim_start().starts_with("tAtm"))
        .ok_or("ATMOSPH.IN header")?
        + 1;
    let mut atm = Vec::new();
    for line in &atm_lines[first..] {
        let v = nums(line);
        if v.len() != 11 {
            break;
        }
        atm.push(v);
    }
    let h_crit_a = -atm[0][4].abs();
    let (meteo_config, meteo_records) =
        parse_meteo_deck(&meteo_text, "Meteo.in").map_err(err("Meteo.in"))?;
    assert_eq!(atm.len(), meteo_records.len(), "records per file");

    for a in &atm {
        assert_eq!(a[4].abs(), -h_crit_a, "one hCritA for the run");
    }
    let atm_records: Vec<AtmosphericRecord> = atm
        .iter()
        .map(|a| AtmosphericRecord {
            t_atm: a[0],
            precipitation: a[1],
        })
        .collect();
    let forcing = MetForcing::new(&meteo_config, &meteo_records, &atm_records, X_CONV, T_CONV)
        .map_err(err("forcing"))?;
    let records = forcing.records().to_vec();
    let schedule = forcing
        .schedule(ForcingUnits {
            time: "h",
            water_depth_rate: "cm/h",
        })
        .map_err(err("schedule"))?;

    let analytic = VanGenuchtenModel::new(
        VanGenuchtenParameters::new(vg[0], vg[1], vg[2], vg[3], vg[4]).map_err(err("vg"))?,
    )
    .map_err(err("vg model"))?;
    let tables = HydraulicTables::new(
        &analytic,
        -htab[1],
        -htab[0],
        100,
        UnitSymbol::new("cm").unwrap(),
        UnitSymbol::new("cm/h").unwrap(),
        UnitSymbol::new("1/cm").unwrap(),
    )
    .map_err(err("tables"))?;
    let span = SpanModel {
        table: &tables,
        analytic: &analytic,
    };
    let models: [&dyn HydraulicModel; 1] = [&span];
    let initial: Vec<Evaluation> = heads
        .iter()
        .map(|&h| span.evaluate(h).map_err(err("initial")))
        .collect::<Result<_, _>>()?;
    let theta: Vec<f64> = initial.iter().map(|e| e.theta).collect();
    let conductivity: Vec<f64> = initial.iter().map(|e| e.conductivity).collect();
    let capacity: Vec<f64> = initial.iter().map(|e| e.capacity).collect();
    let mesh = Mesh1D::new(x.clone(), vec![0; n], None, 1, 0, cos_alpha).map_err(err("mesh"))?;
    let material = TempMaterial::from_tpar(tpar);
    let heat_input = HeatConfigInput {
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
    };
    let t_bot = bc[3];
    // `SetMeteo` receives `tTop` in its `TAver` slot (`HYDRUS.FOR:592-612` -> `TIME.FOR:519`), so
    // with lMeteo the Dirichlet top temperature is the record's `(TMax+TMin)/2`, held over the
    // record's interval (`MetForcing::top_temperatures`); the driver swaps it in per step.
    let top_temperatures = forcing.top_temperatures().to_vec();
    let config = VaporConfig::new(VaporConfigInput {
        heat: heat_input,
        water_enabled: true,
        heat_enabled: true,
        i_enhanc: 1,
        hysteresis: 0,
        chemistry: false,
        density_coupling: false,
        centrifugal: false,
        top_temperature: top_temperatures[0],
        bottom_temperature: t_bot,
        x_conv: X_CONV,
        t_conv: T_CONV,
    })
    .map_err(err("vapor config"))?;
    let policy = WaterStepPolicy {
        max_iterations: tols[0] as usize,
        head_tolerance: tols[2],
        water_tolerance: tols[1],
        minimum_timestep: dt_min,
        retry_factor: 3.0,
        minimum_pivot: MINIMUM_PIVOT,
    };
    let primitive = VaporPrimitiveInput {
        heads: &heads,
        temperatures: &temps,
        liquid_water_contents: &theta,
        liquid_conductivities: &conductivity,
        liquid_capacities: &capacity,
        material_indices: &indices,
        saturated_water_contents: &[vg[1]],
        x_conv: X_CONV,
        t_conv: T_CONV,
    };
    let mut driver = VaporWaterHeatDriver::new(
        config,
        VaporStateInput {
            primitive,
            coordinates: &x,
            gravity: cos_alpha,
            initial_head_predictor: &heads,
            initialization_timestep: dt,
            time: 0.0,
        },
        &indices,
        &[material],
    )
    .map_err(err("vapor driver"))?;
    let source = [VaporSourceMaterial {
        model: &analytic,
        table: Some(&tables),
    }];
    let sink = vec![0.0; n];
    let lower = LowerBoundaryMode::FixedHead {
        head: heads[0],
        row: row(heads[0]),
        flux: 0.0,
    };
    let mut water = WaterState::new(0.0, heads.clone(), theta.clone()).map_err(err("water"))?;
    let mut workspace = WaterWorkspace::new(n).map_err(err("workspace"))?;
    let coupling = VaporAtmosphericCoupling::new(X_CONV).map_err(err("coupling"))?;
    let mut atmo = VaporAtmosphericState::new();
    if let Some(h) = options.top_head_override {
        atmo.kod_top = 4;
        atmo.h_top = h;
        atmo.v_top = 0.01; // below rTop, so Shift keeps head mode at the start
    }

    // Initial NodOut (HYDRUS.FOR:365).
    let props = kernels::evaluate(&primitive).map_err(err("kernels"))?;
    let obs0 = observe_vapor(&VaporObservationInput {
        time: 0.0,
        coordinates: &x,
        heads: &heads,
        temperatures: &temps,
        liquid_conductivities: &conductivity,
        liquid_thermal_conductivities: &props.liquid_thermal_conductivities,
        vapor_head_conductivities: &props.vapor_head_conductivities,
        vapor_thermal_conductivities: &props.vapor_thermal_conductivities,
        top_liquid_new: theta[n - 1],
        top_liquid_old: theta[n - 1],
        top_sink: 0.0,
        timestep: dt,
        gravity: cos_alpha,
    })
    .map_err(err("initial observation"))?;
    let mut run = MetRun {
        time_points: time_points.clone(),
        node_depths: depths.clone(),
        head: vec![rev(&heads)],
        liquid_moisture: vec![rev(&theta)],
        temperature_c: vec![rev(&temps)],
        vapor: VAPOR_FIELDS.iter().map(|&f| (f, Vec::new())).collect(),
        steps: Vec::new(),
        records,
        dt,
    };
    for (name, r) in vapor_rows(&obs0, n) {
        run.vapor.get_mut(name).unwrap().push(r);
    }

    let steps_per_unit = (1.0 / dt).round(); // exact grid: t_k = k / steps_per_unit
    let mut next_print = 1usize;
    let mut k = 0usize;
    while next_print < time_points.len() {
        if options.max_steps.is_some_and(|m| k >= m) {
            break;
        }
        k += 1;
        let t_new = k as f64 / steps_per_unit;
        let step_dt = t_new - water.time();
        let input = WaterStepInput {
            mesh: &mesh,
            materials: &models,
            sink: &sink,
            root_uptake: None,
            timestep: step_dt,
            target_time: t_new,
            face_averaging: FaceAveraging::Arithmetic,
            lower,
            upper: UpperBoundary::Atmospheric {
                critical_head: h_crit_a,
            },
        };
        let t_top = top_temperatures[atmo.event_cursor];
        let report = driver
            .step_atmospheric(
                &mut water,
                &mut workspace,
                input,
                policy,
                &schedule,
                atmo,
                coupling,
                t_top,
                &source,
                CoupledHeatInput {
                    top_temperature: t_top,
                    bottom_temperature: t_bot,
                    x_conv: X_CONV,
                    t_conv: T_CONV,
                    minimum_pivot: MINIMUM_PIVOT,
                },
            )
            .map_err(|e| format!("step {k}: {e:?}"))?;
        let step = &report.step;
        atmo = report.state;
        let fluxes = t_level_fluxes_for_step(driver.state(), step, &x, &sink, cos_alpha)
            .map_err(|e| format!("t_level_fluxes step {k}: {e:?}"))?;
        run.steps.push(StepTrace {
            index: k,
            time: step.accepted_time,
            timestep: step.timestep,
            kod_top: atmo.kod_top,
            h_top: atmo.h_top,
            r_top: atmo.r_top,
            v_top: atmo.v_top,
            top_head: water.heads()[n - 1],
            iterations: [step.passes[0].iterations, step.passes[1].iterations],
            switches: report.switches.clone(),
            below_critical_head: report.below_critical_head,
            liquid_face_fluxes: step.liquid_face_fluxes.clone(),
            vapor_face_fluxes: step.vapor_face_fluxes.clone(),
            total_face_fluxes: step.total_face_fluxes.clone(),
            liquid_contents_old: step.liquid_contents_old.clone(),
            liquid_contents_new: step.liquid_contents_new.clone(),
            vapor_contents_old: step.vapor_contents_old.clone(),
            vapor_contents_new: step.vapor_contents_new.clone(),
            v_top_reported: fluxes.v_top,
            v_bot_reported: fluxes.v_bot,
            v_top_liquid_reported: fluxes.v_top_liquid,
            v_top_vapor_reported: fluxes.v_top_vapor,
        });

        if (water.time() - time_points[next_print]).abs() < 1e-9 {
            let obs = observe_step(driver.state(), step, &x, &sink, cos_alpha)
                .map_err(|e| format!("observe: {e:?}"))?;
            for (name, r) in vapor_rows(&obs, n) {
                run.vapor.get_mut(name).unwrap().push(r);
            }
            run.head.push(rev(water.heads()));
            run.liquid_moisture.push(rev(water.water_contents()));
            run.temperature_c
                .push(rev(driver.state().snapshot().temperatures));
            next_print += 1;
        }
    }
    Ok(run)
}

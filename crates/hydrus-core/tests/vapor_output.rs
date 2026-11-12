//! Vapor `NOD_INF_V` observation: independent hand-computed vectors, not Fortran results.
//!
//! Mesh x=[0,1,3,7] (dx 1,2,4), bottom-first. Values were computed by hand
//! (same-side weights; the opposite-side numbers pin the solver contrast).

use hydrus_core::heat::vapor::driver::{VaporPassReport, VaporStepReport};
use hydrus_core::heat::vapor::output::{observe_step, observe_vapor, VaporObservationInput};
use hydrus_core::heat::vapor::state::{VaporState, VaporStateInput};
use hydrus_core::heat::vapor::{VaporError, VaporPrimitiveInput};
use hydrus_core::{HydraulicModel, VanGenuchtenModel, VanGenuchtenParameters};

const X: [f64; 4] = [0.0, 1.0, 3.0, 7.0];
const H: [f64; 4] = [0.0, -2.0, -8.0, -10.0];
const T: [f64; 4] = [10.0, 12.0, 18.0, 20.0];
const CON: [f64; 4] = [1.0, 2.0, 3.0, 4.0];
const LT: [f64; 4] = [9.0, 9.0, 9.0, 9.0];
const VH: [f64; 4] = [1.0, 3.0, 5.0, 7.0];
const VT: [f64; 4] = [2.0, 2.0, 4.0, 6.0];

fn input(new: f64, old: f64) -> VaporObservationInput<'static> {
    VaporObservationInput {
        time: 2.5,
        coordinates: &X,
        heads: &H,
        temperatures: &T,
        liquid_conductivities: &CON,
        liquid_thermal_conductivities: &LT,
        vapor_head_conductivities: &VH,
        vapor_thermal_conductivities: &VT,
        top_liquid_new: new,
        top_liquid_old: old,
        top_sink: 0.5,
        timestep: 0.5,
        gravity: 1.0,
    }
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-12, "{a} vs {b}");
}

#[test]
fn rows_are_top_first_with_source_node_depth_and_time() {
    let o = observe_vapor(&input(0.30, 0.28)).unwrap();
    assert_eq!(o.time, 2.5);
    let ids: Vec<_> = o.rows.iter().map(|r| (r.node, r.rust_index)).collect();
    assert_eq!(ids, [(1, 3), (2, 2), (3, 1), (4, 0)]);
    let depths: Vec<_> = o.rows.iter().map(|r| r.depth).collect();
    assert_eq!(depths, [0.0, -4.0, -6.0, -7.0]);
    for r in &o.rows {
        let i = r.rust_index;
        assert_eq!(
            (r.con, r.con_lt, r.con_vh, r.con_vt),
            (CON[i], LT[i], VH[i], VT[i])
        );
        near(r.v_total, r.v_liquid + r.v_vapor);
        near(r.v_vapor, r.v_vap_iso + r.v_vap_term);
    }
}

#[test]
fn interior_uses_same_side_weights_not_solver_opposite_side() {
    let o = observe_vapor(&input(0.30, 0.28)).unwrap();
    let r1 = o.rows[2]; // rust index 1
    near(r1.v_liquid, 3.8333333333333335);
    near(r1.v_vap_iso, 9.333333333333334);
    near(r1.v_vap_term, -7.333333333333333);
    let r2 = o.rows[1]; // rust index 2
    near(r2.v_liquid, 0.5);
    near(r2.v_vap_iso, 6.0);
    near(r2.v_vap_term, -4.666666666666667);
    // Opposite-side (Veloc) values differ materially, so the mapping is detectable.
    assert!((r1.v_liquid - 2.6666666666666665).abs() > 1.0);
    assert!((r2.v_liquid - 2.75).abs() > 1.0);
    assert!((r2.v_vap_iso - 9.0).abs() > 1.0);
}

#[test]
fn top_has_liquid_storage_and_sink_bottom_has_none_vapor_never() {
    let o = observe_vapor(&input(0.30, 0.28)).unwrap();
    let (top, bottom) = (o.rows[0], o.rows[3]);
    // top liquid: -1.75 - (0.02)*4/2/0.5 - 0.5*4/2 = -2.83; gravity in liquid only
    near(top.v_liquid, -2.83);
    near(top.v_vap_iso, 3.0);
    near(top.v_vap_term, -2.5);
    near(top.v_vapor, 0.5);
    near(top.v_total, -2.33);
    near(bottom.v_liquid, 1.5);
    near(bottom.v_vap_iso, 4.0);
    near(bottom.v_vap_term, -4.0);
    near(bottom.v_vapor, 0.0);
    // Equal contents (initial print): storage vanishes, sink remains; vapor unchanged.
    let z = observe_vapor(&input(0.28, 0.28)).unwrap();
    near(z.rows[0].v_liquid, -2.75);
    assert_eq!(z.rows[0].v_vapor, top.v_vapor);
    assert_eq!(z.rows[3], bottom);
}

#[test]
fn conlt_is_echoed_but_does_not_enter_liquid_flux() {
    let mut other = LT;
    other[1] = 1.0e6;
    let mut i = input(0.30, 0.28);
    let a = observe_vapor(&i).unwrap();
    i.liquid_thermal_conductivities = &other;
    let b = observe_vapor(&i).unwrap();
    for (p, q) in a.rows.iter().zip(&b.rows) {
        assert_eq!(p.v_liquid, q.v_liquid);
    }
    assert_eq!(b.rows[2].con_lt, 1.0e6);
}

#[test]
fn rejects_bad_inputs() {
    let short = [1.0, 2.0];
    let mut i = input(0.3, 0.28);
    i.heads = &short;
    assert!(matches!(
        observe_vapor(&i),
        Err(VaporError::LengthMismatch { name: "heads", .. })
    ));
    let mut i = input(0.3, 0.28);
    i.timestep = 0.0;
    assert!(matches!(
        observe_vapor(&i),
        Err(VaporError::InvalidValue { .. })
    ));
    let nan = [0.0, f64::NAN, 1.0, 2.0];
    let mut i = input(0.3, 0.28);
    i.temperatures = &nan;
    assert!(matches!(
        observe_vapor(&i),
        Err(VaporError::NonFinite { .. })
    ));
}

#[test]
fn observe_step_reads_snapshot_and_report_without_mutation() {
    let model =
        VanGenuchtenModel::new(VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0e-5).unwrap())
            .unwrap();
    let e = model.evaluate(-100.0).unwrap();
    let heads = H;
    let state = VaporState::new(VaporStateInput {
        primitive: VaporPrimitiveInput {
            heads: &heads,
            temperatures: &T,
            liquid_water_contents: &[e.theta; 4],
            liquid_conductivities: &[e.conductivity; 4],
            liquid_capacities: &[e.capacity; 4],
            material_indices: &[0; 4],
            saturated_water_contents: &[0.45],
            x_conv: 100.0,
            t_conv: 86400.0,
        },
        coordinates: &X,
        gravity: 1.0,
        initial_head_predictor: &heads,
        initialization_timestep: 0.5,
        time: 2.5,
    })
    .unwrap();
    let pass = VaporPassReport {
        timestep: 0.5,
        iterations: 1,
        temperature_estimate: vec![0.0; 4],
        initial_heads: vec![0.0; 4],
        initial_material_heads: vec![0.0; 4],
        heads: vec![0.0; 4],
        temperatures: vec![0.0; 4],
        heat_flux: 0.0,
        minimum_absolute_pivot: 1.0,
    };
    let report = VaporStepReport {
        accepted_time: 2.5,
        timestep: 0.5,
        restarts: 0,
        passes: [pass.clone(), pass],
        heat_flux: 0.0,
        minimum_absolute_pivot: 1.0,
        liquid_contents_old: vec![0.1, 0.1, 0.1, 0.28],
        liquid_contents_new: vec![0.9, 0.9, 0.9, 0.30],
        vapor_contents_old: vec![0.0; 4],
        vapor_contents_new: vec![0.0; 4],
        liquid_conductivities: CON.to_vec(),
        liquid_thermal_conductivities: LT.to_vec(),
        vapor_head_conductivities: VH.to_vec(),
        vapor_thermal_conductivities: VT.to_vec(),
        pre_solve_heads: vec![7.0; 4],
        pre_solve_liquid_water_contents: vec![7.0; 4],
        effective_hydraulic_capacities: vec![7.0; 4],
        liquid_velocities: vec![7.0; 4],
        vapor_velocities: vec![7.0; 4],
        liquid_face_fluxes: vec![7.0; 3],
        vapor_face_fluxes: vec![7.0; 3],
        total_face_fluxes: vec![7.0; 3],
        temperatures: T.to_vec(),
    };
    let (state0, report0) = (state.clone(), report.clone());
    let sink = [0.0, 0.0, 0.0, 0.5];
    let o = observe_step(&state, &report, &X, &sink, 1.0).unwrap();
    assert_eq!(o, observe_vapor(&input(0.30, 0.28)).unwrap());
    assert_eq!(state, state0);
    assert_eq!(report, report0);
    let mut stale = report.clone();
    stale.accepted_time = 2.0;
    assert!(matches!(
        observe_step(&state, &stale, &X, &sink, 1.0),
        Err(VaporError::InvalidValue { value, .. }) if value == -0.5
    ));
    let mut wrong_dt = report.clone();
    wrong_dt.timestep = 0.25;
    assert!(observe_step(&state, &wrong_dt, &X, &sink, 1.0).is_err());
    assert_eq!(state, state0);
    // Solver velocities in the report are not substituted.
    assert!(o.rows.iter().all(|r| r.v_liquid != 7.0 && r.v_vapor != 7.0));
}

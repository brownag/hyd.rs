//! Water balance over the met-switched run of `vapor_met_bare_soil_on` (240 h, 2400 steps,
//! flux/head switches at 42.7, 120.1, 200.3 h).
//!
//! Residual definition follows `vapor_closed_pair_balance.rs:189-197` and
//! `vapor_free_drainage_balance.rs:189-198`: `|dStorage - netIn|`, `dStorage` the trapezoid-weighted
//! change of liquid plus vapor content, tolerance `max(1e-9 * |netIn|, 1e-13)`.
//!
//! Boundary fluxes. `upper_for` (`atmospheric.rs:486`) and the driver's `FixedHead`
//! (`support/vapor_met.rs:505`) pass flux 0.0, so `total_face_fluxes` at a head end is NOT a
//! solved boundary flux; no assertion here uses it. Head ends use the `vTop` form
//! (`WATFLOW.FOR:395-407`), which is also how Fortran's own balance defines head-boundary fluxes.
//! Only the flux-mode top (the prescribed flux) is independent. Measured: the prescribed flux
//! already is the total (liquid plus vapor) top flux, because the top row of the solve carries the
//! vapor terms inside the interior face flux; adding `vapor_face_fluxes[n]` on top of it
//! double-counts and fails the precedent tolerance by about 5e7 x (worst at the rain record).
#[path = "support/vapor_met.rs"]
mod vapor_met;
use std::sync::OnceLock;
use vapor_met::{MetRun, RunOptions, StepTrace};

const REL_TOL: f64 = 1e-9;
const ABS_FLOOR: f64 = 1e-13;
const FORTRAN_PRINT_TIMES: [f64; 11] = [
    0.0, 24.0, 48.0, 72.0, 96.0, 120.0, 144.0, 168.0, 192.0, 216.0, 240.0,
];

fn run() -> &'static (MetRun, Vec<f64>) {
    static RUN: OnceLock<(MetRun, Vec<f64>)> = OnceLock::new();
    RUN.get_or_init(|| {
        let r = vapor_met::run_met_case(RunOptions::default()).expect("met run");
        let mut x = r.node_depths.clone();
        x.reverse(); // bottom first
        let n = x.len();
        let w = (0..n)
            .map(|i| {
                if i == 0 {
                    (x[1] - x[0]) / 2.0
                } else if i == n - 1 {
                    (x[n - 1] - x[n - 2]) / 2.0
                } else {
                    (x[i + 1] - x[i - 1]) / 2.0
                }
            })
            .collect();
        (r, w)
    })
}

fn dc(s: &StepTrace, i: usize) -> f64 {
    (s.liquid_contents_new[i] + s.vapor_contents_new[i])
        - (s.liquid_contents_old[i] + s.vapor_contents_old[i])
}

fn d_storage(s: &StepTrace, w: &[f64]) -> f64 {
    (0..w.len()).map(|i| w[i] * dc(s, i)).sum()
}

/// Bottom inflow-minus-top-outflow (cm/h, positive up) by the `vTop` form at both ends.
///
/// Uses the water solve's `total_face_fluxes` (evaluated at pre-heat-solve temperatures) rather
/// than the post-`Temper` `TLInf` fluxes (`v_top_reported`/`v_bot_reported`), so this closes the
/// water solve's `d_storage` to machine precision; post-`Temper` `TLInf` parity is tested in
/// `vapor_met_tlevel_parity`.
fn net_vtop(s: &StepTrace, w: &[f64], drop_vapor_storage: bool) -> f64 {
    let n = w.len();
    let t = s.timestep;
    let top_store = if drop_vapor_storage {
        s.liquid_contents_new[n - 1] - s.liquid_contents_old[n - 1]
    } else {
        dc(s, n - 1)
    };
    let bot = s.total_face_fluxes[1] + w[0] * dc(s, 0) / t;
    let top = s.total_face_fluxes[n - 1] - w[n - 1] * top_store / t;
    bot - top
}

/// Flux-mode: prescribed top flux (`face[n]` liquid entry, the total top flux), bottom by `vTop`
/// form. `add_vapor` adds the solved vapor face entry again (double-count; negative check only).
fn net_flux_mode(s: &StepTrace, w: &[f64], top_scale: f64, add_vapor: bool) -> f64 {
    let n = w.len();
    let bot = s.total_face_fluxes[1] + w[0] * dc(s, 0) / s.timestep;
    let top = s.liquid_face_fluxes[n] * top_scale
        + if add_vapor {
            s.vapor_face_fluxes[n]
        } else {
            0.0
        };
    bot - top
}

fn tol(net_in: f64) -> f64 {
    (REL_TOL * net_in.abs()).max(ABS_FLOOR)
}

/// `(max residual, max residual / precedent tol)` over steps selected by `kod_top` sign.
fn worst(f: impl Fn(&StepTrace) -> f64, head_mode: bool) -> (f64, f64) {
    let (r, w) = run();
    let mut out = (0.0_f64, 0.0_f64);
    for s in r.steps.iter().filter(|s| (s.kod_top > 0) == head_mode) {
        let net = f(s) * s.timestep;
        let res = (d_storage(s, w) - net).abs();
        out.0 = out.0.max(res);
        out.1 = out.1.max(res / tol(net));
    }
    out
}

#[test]
fn run_geometry_and_switch_guards() {
    let (r, w) = run();
    assert_eq!(r.steps.len(), 2400);
    assert!(r.steps.iter().all(|s| (s.timestep - 0.1).abs() < 1e-12));
    let sw: Vec<(f64, i8)> = r.kod_changes().iter().map(|c| (c.time, c.to)).collect();
    assert_eq!(sw.len(), 3);
    for (got, want) in sw.iter().zip([(42.7, 4), (120.1, -4), (200.3, 4)]) {
        assert!((got.0 - want.0).abs() < 1e-6 && got.1 == want.1, "{sw:?}");
    }
    assert!((w.iter().sum::<f64>() - 40.0).abs() < 1e-9);
    let flux = r.steps.iter().filter(|s| s.kod_top < 0).count();
    assert!(flux > 500 && 2400 - flux > 500, "both modes visited");
    assert!(r
        .steps
        .iter()
        .any(|s| net_vtop(s, w, false).abs() * s.timestep > 1e-3));
}

/// Interior-conservation check at the ends, NOT a boundary-flux test: both end fluxes are the
/// `vTop` form (interior face plus half-cell storage, `WATFLOW.FOR:395-407`), so this closes by
/// construction. It verifies cell-to-cell conservation of liquid plus vapor in flux and head mode.
/// Fortran's balance defines head-boundary fluxes the same way.
#[test]
fn interior_conservation_with_vtop_end_fluxes_at_precedent_tolerance() {
    let (r, w) = run();
    let _ = r;
    for head in [false, true] {
        let (abs, rel) = worst(|s| net_vtop(s, w, false), head);
        println!("vTop ends head_mode={head}: max resid {abs:e}, max resid/tol {rel:e}");
        assert!(rel <= 1.0, "head_mode={head}: resid/tol {rel:e}");
    }
}

/// Negative checks, all against the precedent tolerance: dropping the top vapor storage or scaling
/// the end flux breaks the vTop closure; scaling the prescribed top flux, or adding the vapor face
/// entry on top of it, breaks the flux-mode closure.
#[test]
fn negative_checks_detect_dropped_vapor_and_perturbed_flux() {
    let (_, w) = run();
    let (_, rel) = worst(|s| net_vtop(s, w, true), false);
    assert!(
        rel > 1.0,
        "dropping vapor storage must fail the closure: {rel:e}"
    );
    let (_, rel) = worst(|s| net_vtop(s, w, false) * 1.01, true);
    assert!(rel > 1.0, "perturbed flux must fail the closure: {rel:e}");
    let (_, rel) = worst(|s| net_flux_mode(s, w, 1.01, false), false);
    assert!(
        rel > 1.0,
        "perturbed prescribed top flux must fail: {rel:e}"
    );
    let (abs, rel) = worst(|s| net_flux_mode(s, w, 1.0, true), false);
    println!("flux mode with vapor face entry added: max resid {abs:e}, resid/tol {rel:e}");
    assert!(rel > 1.0, "double-counted vapor term must fail: {rel:e}");
}

/// Flux-mode top, independent: prescribed top flux against trapezoid storage, bottom by the
/// `vTop` form, at the precedent tolerance. Covers the rain record and the switch steps.
#[test]
fn flux_mode_top_balance_at_precedent_tolerance() {
    let (_, w) = run();
    let (abs, rel) = worst(|s| net_flux_mode(s, w, 1.0, false), false);
    println!("flux mode: max resid {abs:e}, max resid/precedent tol {rel:e}");
    assert!(rel <= 1.0, "resid/tol {rel:e}");
}

/// Cumulative and per-print-time residuals (flux mode: prescribed top; head mode: vTop form),
/// next to Fortran `BALANCE.OUT` WatBalT (evidence from the untracked pinned run, hard-coded).
#[test]
fn cumulative_residuals_by_print_time_report() {
    let (r, w) = run();
    let fortran = [
        f64::NAN,
        1.0520e-4,
        1.0610e-4,
        1.0550e-4,
        1.0598e-4,
        1.0490e-4,
        1.0255e-4,
        1.0081e-4,
        1.0014e-4,
        9.656e-5,
        9.9182e-5,
    ];
    let (mut cum_ds, mut cum_net, mut cum_vt, mut next) = (0.0, 0.0, 0.0, 1);
    for s in &r.steps {
        let t = s.timestep;
        cum_ds += d_storage(s, w);
        cum_vt += net_vtop(s, w, false) * t;
        cum_net += if s.kod_top > 0 {
            net_vtop(s, w, false)
        } else {
            net_flux_mode(s, w, 1.0, false)
        } * t;
        if (s.time - FORTRAN_PRINT_TIMES[next]).abs() < 1e-6 {
            println!(
                "t={:>5}: Rust cumulative resid {:e} (vTop ends {:e}); Fortran WatBalT {:e}",
                s.time,
                (cum_ds - cum_net).abs(),
                (cum_ds - cum_vt).abs(),
                fortran[next]
            );
            assert!((cum_ds - cum_vt).abs() <= (REL_TOL * cum_vt.abs()).max(ABS_FLOOR) + 1e-12);
            next += 1;
        }
    }
    assert_eq!(next, 11);
}

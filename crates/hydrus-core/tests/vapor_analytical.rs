//! Independent mathematical references for the vapor kernels, water rows, phase velocities and
//! `TempAdj` terms (ADR 0009). No Fortran or candidate-vs-reference comparison, and no analytical
//! solution of the nonlinear coupled problem is claimed.
//!
//! Predeclared roundoff bounds (not acceptance thresholds; f64 epsilon = 2.2e-16):
//! - `KERNEL_REL = 1e-12` (~4500 eps): kernels chain about 40 flops with `exp`/`powf` of arguments
//!   up to about 30 (error about 30 eps each) and a one-kelvin density difference that cancels by
//!   a factor of about 15.
//! - `ROW_REL = 1e-14` (~45 eps): rows and velocities are at most about 10 flops of same-sign
//!   or mildly cancelling terms.
//! - `ADJUST_REL = 1e-13` (~450 eps): `TempAdj` sums about 20 products including a density
//!   polynomial.
//! - `CAP_RATIO_REL = 1e-10`: the vapor part of the capacity is about 1.4e-3 of the total, so
//!   extracting it amplifies roundoff by about 700 (about 1.6e-13 expected).
//!
//! Time conversion follows `INPUT.FOR:186-197`: `tConv = 1/3600` for hours, `1/86400` for days.
//!
//! Reference literals for the kernels were produced with 60-digit `mpmath` evaluation of the
//! contract formulas (`planning/master/M6_VAPOR_CONTRACT.md`), then rounded to f64.

use hydrus_core::heat::vapor::adjust::{adjust_heat_rows, VaporHeatAdjustmentInput};
use hydrus_core::heat::vapor::kernels;
use hydrus_core::heat::vapor::water::{
    assemble_water_rows, phase_velocities, PhaseVelocityInput, VaporWaterRowsInput,
};
use hydrus_core::heat::vapor::{VaporPrimitiveInput, VaporProperties};

const KERNEL_REL: f64 = 1.0e-12;
const ROW_REL: f64 = 1.0e-14;
const ADJUST_REL: f64 = 1.0e-13;
const CAP_RATIO_REL: f64 = 1.0e-10;

fn close(actual: f64, expected: f64, rel: f64, what: &str) {
    let tol = rel * expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        (actual - expected).abs() <= tol,
        "{what}: {actual:e} vs {expected:e} (tol {tol:e})"
    );
}

struct Node {
    h: f64,
    t: f64,
    theta: f64,
    ts: f64,
    k: f64,
    c: f64,
    xc: f64,
    tc: f64,
}

fn kernel(n: &Node) -> VaporProperties {
    // The kernel requires at least three nodes: three identical nodes, read node 1.
    kernels::evaluate(&VaporPrimitiveInput {
        heads: &[n.h; 3],
        temperatures: &[n.t; 3],
        liquid_water_contents: &[n.theta; 3],
        liquid_conductivities: &[n.k; 3],
        liquid_capacities: &[n.c; 3],
        material_indices: &[0; 3],
        saturated_water_contents: &[n.ts],
        x_conv: n.xc,
        t_conv: n.tc,
    })
    .unwrap()
}

/// A1. Assumptions: enhancement on (`iEnhanc=1`), positive air content, `h<0`, one node, source
/// grouping with SI conversion at the source sites, literals from the contract formulas.
#[test]
fn a1_kernels_match_high_precision_hand_references() {
    let a = Node {
        h: -3000.0,
        t: 20.0,
        theta: 0.12,
        ts: 0.45,
        k: 2.0e-6,
        c: 3.0e-5,
        xc: 100.0,
        tc: 1.0 / 3600.0,
    };
    let p = kernel(&a);
    close(
        p.vapor_head_conductivities[1],
        1.350690937541982e-09,
        KERNEL_REL,
        "ConVh a",
    );
    close(
        p.vapor_thermal_conductivities[1],
        0.0011542971102944516,
        KERNEL_REL,
        "ConVT a",
    );
    close(
        p.liquid_thermal_conductivities[1],
        8.884907497565725e-05,
        KERNEL_REL,
        "ConLT a",
    );
    close(
        p.vapor_contents[1],
        5.7020284905738365e-06,
        KERNEL_REL,
        "ThetaV a",
    );
    close(
        p.effective_hydraulic_capacities[1],
        3.0040827561211118e-05,
        KERNEL_REL,
        "Ceff a",
    );
    let b = Node {
        h: -8000.0,
        t: 35.0,
        theta: 0.07,
        ts: 0.41,
        k: 1.0e-7,
        c: 8.0e-6,
        xc: 100.0,
        tc: 1.0 / 86400.0,
    };
    let p = kernel(&b);
    close(
        p.vapor_head_conductivities[1],
        1.0407775752169423e-07,
        KERNEL_REL,
        "ConVh b",
    );
    close(
        p.vapor_thermal_conductivities[1],
        0.08058747007868543,
        KERNEL_REL,
        "ConVT b",
    );
    close(
        p.liquid_thermal_conductivities[1],
        1.2406231742940604e-05,
        KERNEL_REL,
        "ConLT b",
    );
    close(
        p.vapor_contents[1],
        1.348462521836513e-05,
        KERNEL_REL,
        "ThetaV b",
    );
    close(
        p.effective_hydraulic_capacities[1],
        8.092701349684409e-06,
        KERNEL_REL,
        "Ceff b",
    );
}

/// A5. The source capacity multiplies the vapor term by `xConv`; the continuum derivative of
/// `theta_v(h)` with `h_m=h/xConv` divides by it. The kernel follows the source literally, so the
/// vapor part exceeds the continuum value by exactly `xConv^2`. Reported discrepancy, asserted only
/// as the literal source form.
#[test]
fn a5_capacity_follows_source_literal_conversion_not_continuum() {
    let a = Node {
        h: -3000.0,
        t: 20.0,
        theta: 0.12,
        ts: 0.45,
        k: 2.0e-6,
        c: 3.0e-5,
        xc: 100.0,
        tc: 1.0 / 3600.0,
    };
    let p = kernel(&a);
    let liquid_part = 2.9999481633773586e-05;
    let vapor_literal = 4.1345927437532584e-08;
    let vapor_continuum = 4.1345927437532585e-12;
    let part = p.effective_hydraulic_capacities[1] - liquid_part;
    close(
        part,
        vapor_literal,
        CAP_RATIO_REL,
        "literal vapor capacity part",
    );
    close(
        part / vapor_continuum,
        a.xc * a.xc,
        CAP_RATIO_REL,
        "literal/continuum = xConv^2",
    );
    println!(
        "capacity discrepancy: literal vapor part {part:e}, continuum {vapor_continuum:e}, ratio {:e}",
        part / vapor_continuum
    );
}

// Fixed seeded inputs: nonuniform 4-node mesh (cm), values from a SplitMix64 stream with seed
// 0x2562, rounded to 3-4 digits and frozen here.
const X: [f64; 4] = [0.0, 7.3, 21.9, 30.4];
const KL: [f64; 4] = [2.1e-6, 1.7e-6, 1.1e-6, 0.9e-6];
const VH: [f64; 4] = [3.0e-9, 2.2e-9, 1.4e-9, 1.1e-9];
const VT: [f64; 4] = [4.0e-8, 3.1e-8, 2.6e-8, 2.0e-8];
const CAPS: [f64; 4] = [3.1e-5, 2.9e-5, 2.2e-5, 2.0e-5];
const HS: [f64; 4] = [-3100.0, -3350.5, -3720.25, -3999.0];
const TH_OLD: [f64; 4] = [0.101, 0.099, 0.092, 0.090];
const TH_NEW: [f64; 4] = [0.1004, 0.0987, 0.0915, 0.0893];
const TV_OLD: [f64; 4] = [5.0e-6, 4.6e-6, 4.1e-6, 3.9e-6];
const TV_NEW: [f64; 4] = [5.3e-6, 4.8e-6, 4.5e-6, 4.0e-6];
const DT: f64 = 0.05;

fn props(vh: [f64; 4], vt: [f64; 4], tv: [f64; 4]) -> VaporProperties {
    VaporProperties {
        liquid_conductivities: KL.to_vec(),
        liquid_thermal_conductivities: vec![0.0; 4],
        vapor_head_conductivities: vh.to_vec(),
        vapor_thermal_conductivities: vt.to_vec(),
        vapor_contents: tv.to_vec(),
        effective_hydraulic_capacities: CAPS.to_vec(),
    }
}

fn rows(
    g: f64,
    temps: &[f64; 4],
    p: &VaporProperties,
    tv_old: &[f64; 4],
    tv_new: &[f64; 4],
) -> Vec<(f64, f64, f64, f64)> {
    assemble_water_rows(&VaporWaterRowsInput {
        coordinates: &X,
        gravity: g,
        liquid_conductivities: &KL,
        effective_capacities: &CAPS,
        heads_new: &HS,
        liquid_water_old: &TH_OLD,
        liquid_water_new: &TH_NEW,
        vapor_water_old: tv_old,
        vapor_water_new: tv_new,
        temperatures: temps,
        sink: &[0.0, 2.0e-7, 1.0e-7, 0.0],
        timestep: DT,
        vapor: p,
    })
    .unwrap()
    .into_iter()
    .map(|r| (r.lower, r.diagonal, r.upper, r.rhs))
    .collect()
}

/// Hand row for node `i`, written with explicit face sums (no shared helper with the product code).
/// `thermal`: include the ConVT temperature-gradient divergence; `vapor_storage`: include it.
fn hand_row(
    i: usize,
    g: f64,
    temps: &[f64; 4],
    vh: &[f64; 4],
    vt: &[f64; 4],
    tvo: &[f64; 4],
    tvn: &[f64; 4],
) -> (f64, f64, f64, f64) {
    let sink = [0.0, 2.0e-7, 1.0e-7, 0.0];
    let dxa = X[i] - X[i - 1];
    let dxb = X[i + 1] - X[i];
    let w = 0.5 * (X[i + 1] - X[i - 1]);
    let kla = 0.5 * (KL[i] + KL[i - 1]);
    let klb = 0.5 * (KL[i] + KL[i + 1]);
    let ca = kla + 0.5 * (vh[i] + vh[i - 1]);
    let cb = klb + 0.5 * (vh[i] + vh[i + 1]);
    let cap = CAPS[i] * w / DT;
    let rhs = cap * HS[i] - (TH_NEW[i] - TH_OLD[i]) * w / DT - (kla - klb) * g - sink[i] * w
        + 0.5 * (vt[i] + vt[i + 1]) * (temps[i + 1] - temps[i]) / dxb
        - 0.5 * (vt[i] + vt[i - 1]) * (temps[i] - temps[i - 1]) / dxa
        - (tvn[i] - tvo[i]) * w / DT;
    (-ca / dxa, ca / dxa + cb / dxb + cap, -cb / dxb, rhs)
}

fn compare_rows(got: &[(f64, f64, f64, f64)], want: &[(f64, f64, f64, f64)]) {
    for (r, (g, w)) in got.iter().zip(want).enumerate() {
        close(g.0, w.0, ROW_REL, &format!("lower {r}"));
        close(g.1, w.1, ROW_REL, &format!("diag {r}"));
        close(g.2, w.2, ROW_REL, &format!("upper {r}"));
        // RHS terms mix large `Cap*h` with small storage; compare against the largest RHS term.
        let big = (CAPS[r + 1] * 0.5 * (X[r + 2] - X[r]) / DT * HS[r + 1]).abs();
        assert!(
            (g.3 - w.3).abs() <= ROW_REL * big,
            "rhs {r}: {:e} vs {:e}",
            g.3,
            w.3
        );
    }
}

/// A2. Assumptions: uniform temperature (vapor thermal term vanishes), given heads/contents,
/// gravity on liquid only, vapor head conductivity added to diffusion faces only.
#[test]
fn a2_isothermal_head_gradient_rows_and_velocities() {
    let t = [20.0; 4];
    let p = props(VH, VT, TV_NEW);
    for g in [0.0, 1.0] {
        let got = rows(g, &t, &p, &TV_OLD, &TV_NEW);
        let want: Vec<_> = (1..3)
            .map(|i| hand_row(i, g, &t, &VH, &VT, &TV_OLD, &TV_NEW))
            .collect();
        compare_rows(&got, &want);
    }
    // Phase velocities at interior node 1 (hand, opposite-side spacing weights, `Veloc`).
    let v = |g: f64| {
        phase_velocities(&PhaseVelocityInput {
            coordinates: &X,
            gravity: g,
            heads: &HS,
            liquid_conductivities: &KL,
            liquid_water_old: &TH_OLD,
            liquid_water_new: &TH_NEW,
            sink: &[0.0; 4],
            temperatures: &t,
            vapor_water_old: &TV_OLD,
            vapor_water_new: &TV_NEW,
            timestep: DT,
            vapor: &p,
        })
        .unwrap()
    };
    let (v0, v1) = (v(0.0), v(1.0));
    let i = 1;
    let dxa = X[2] - X[1];
    let dxb = X[1] - X[0];
    let vap =
        |a: usize, b: usize, d: f64| -0.5 * (VH[a] + VH[b]) * (HS[a.max(b)] - HS[a.min(b)]) / d;
    // face with node 2 uses (h2-h1)/dxa; face with node 0 uses (h1-h0)/dxb.
    let want_vap = (vap(1, 2, dxa) * dxb + vap(1, 0, dxb) * dxa) / (dxa + dxb);
    close(v0.vapor[i], want_vap, ROW_REL, "vapor velocity");
    // Gravity changes liquid only.
    assert_eq!(v0.vapor, v1.vapor);
    let liq = |g: f64| {
        let va = -0.5 * (KL[1] + KL[2]) * ((HS[2] - HS[1]) / dxa + g);
        let vb = -0.5 * (KL[1] + KL[0]) * ((HS[1] - HS[0]) / dxb + g);
        (va * dxb + vb * dxa) / (dxa + dxb)
    };
    close(v1.liquid[i], liq(1.0), ROW_REL, "liquid velocity g=1");
    close(v0.liquid[i], liq(0.0), ROW_REL, "liquid velocity g=0");
    for k in 0..4 {
        assert_eq!(v1.total[k], v1.liquid[k] + v1.vapor[k]);
    }
}

/// A3. Assumptions: uniform head and temperature, no gravity, so every conductive/thermal flux is
/// zero; the row residual `lower*h+diag*h+upper*h-rhs` is storage plus sink only.
#[test]
fn a3_zero_gradient_residual_is_storage_only() {
    let h = 1234.5_f64;
    let t = [20.0; 4];
    let p = props(VH, VT, TV_NEW);
    let heads = [-h; 4];
    let sink = [0.0, 2.0e-7, 1.0e-7, 0.0];
    let rows = assemble_water_rows(&VaporWaterRowsInput {
        coordinates: &X,
        gravity: 0.0,
        liquid_conductivities: &KL,
        effective_capacities: &CAPS,
        heads_new: &heads,
        liquid_water_old: &TH_OLD,
        liquid_water_new: &TH_NEW,
        vapor_water_old: &TV_OLD,
        vapor_water_new: &TV_NEW,
        temperatures: &t,
        sink: &sink,
        timestep: DT,
        vapor: &p,
    })
    .unwrap();
    for (r, row) in rows.iter().enumerate() {
        let i = r + 1;
        let w = 0.5 * (X[i + 1] - X[i - 1]);
        let res =
            row.lower * heads[i - 1] + row.diagonal * heads[i] + row.upper * heads[i + 1] - row.rhs;
        let want = ((TH_NEW[i] - TH_OLD[i]) + (TV_NEW[i] - TV_OLD[i])) * w / DT + sink[i] * w;
        // Matrix products are about Cap*w/dt*h ~ 1e-3: absolute bound 100 eps of that.
        let tol = 1.0e-14 * CAPS[i] * w / DT * h + 1.0e-14 * want.abs();
        assert!((res - want).abs() <= tol, "row {r}: {res:e} vs {want:e}");
    }
}

/// A4. Assumptions: `ConVh=ConVT=0` and constant vapor content (zero-vapor limit): rows equal the
/// liquid-only Richards rows and the vapor/total velocities are zero/liquid.
#[test]
fn a4_zero_vapor_reduces_to_liquid_only() {
    let z = [0.0; 4];
    let t = [10.0, 15.0, 22.0, 31.0];
    let p = props(z, z, z);
    let got = rows(1.0, &t, &p, &z, &z);
    for (r, g) in got.iter().enumerate() {
        let i = r + 1;
        let dxa = X[i] - X[i - 1];
        let dxb = X[i + 1] - X[i];
        let w = 0.5 * (X[i + 1] - X[i - 1]);
        let ka = 0.5 * (KL[i] + KL[i - 1]);
        let kb = 0.5 * (KL[i] + KL[i + 1]);
        let cap = CAPS[i] * w / DT;
        let sink = [0.0, 2.0e-7, 1.0e-7, 0.0][i];
        close(g.0, -ka / dxa, ROW_REL, "lower");
        close(g.1, ka / dxa + kb / dxb + cap, ROW_REL, "diag");
        close(g.2, -kb / dxb, ROW_REL, "upper");
        let rhs = cap * HS[i] - (TH_NEW[i] - TH_OLD[i]) * w / DT - (ka - kb) * 1.0 - sink * w;
        assert!((g.3 - rhs).abs() <= ROW_REL * (cap * HS[i]).abs());
    }
    let v = phase_velocities(&PhaseVelocityInput {
        coordinates: &X,
        gravity: 1.0,
        heads: &HS,
        liquid_conductivities: &KL,
        liquid_water_old: &TH_OLD,
        liquid_water_new: &TH_NEW,
        sink: &[0.0; 4],
        temperatures: &t,
        vapor_water_old: &z,
        vapor_water_new: &z,
        timestep: DT,
        vapor: &p,
    })
    .unwrap();
    assert!(v.vapor.iter().all(|&x| x == 0.0));
    assert_eq!(v.total, v.liquid);
}

/// A6. Assumptions: given fixed vapor velocity/content/temperature arrays on a nonuniform mesh;
/// `Lat=rho(T)*(2.501e6-2369.2T)/(xc*tc^2)` evaluated with the contract polynomial in the test;
/// interior velocity gradient carries the source factor two (`TEMPER.FOR:202,211`); endpoints use
/// one-sided differences; level-one latent source uses old `T` and old/new vapor storage change.
#[test]
fn a6_tempadj_terms_match_hand_reference() {
    let to = [12.0, 15.0, 19.0, 23.0];
    let tn = [12.5, 15.8, 19.6, 22.9];
    let vo = [1.0e-9, 2.0e-9, 1.5e-9, 0.5e-9];
    let vn = [1.2e-9, 1.8e-9, 1.7e-9, 0.4e-9];
    let (xc, tc) = (100.0, 1.0 / 3600.0);
    let adj = adjust_heat_rows(&VaporHeatAdjustmentInput {
        coordinates: &X,
        temperature_old: &to,
        temperature_new_estimate: &tn,
        vapor_velocity_old: &vo,
        vapor_velocity_new: &vn,
        vapor_content_old: &TV_OLD,
        vapor_content_new: &TV_NEW,
        timestep: DT,
        x_conv: xc,
        t_conv: tc,
    })
    .unwrap();
    let lat = |t: f64| {
        (1.0 - 7.37e-6 * (t - 4.0) * (t - 4.0) + 3.79e-8 * (t - 4.0) * (t - 4.0) * (t - 4.0))
            * 1000.0
            * (2.501e6 - 2369.2 * t)
            / (xc * tc * tc)
    };
    let grad = |v: &[f64; 4], j: usize| match j {
        0 => (v[1] - v[0]) / (X[1] - X[0]),
        3 => (v[3] - v[2]) / (X[3] - X[2]),
        _ => 2.0 * (v[j + 1] - v[j - 1]) / (X[j + 1] - X[j - 1]),
    };
    let cv = 1.8e6 / (xc * tc * tc);
    let g = |t: &[f64; 4], v: &[f64; 4], j: usize| {
        -lat(t[j]) * (grad(v, j) + (TV_NEW[j] - TV_OLD[j]) / DT)
    };
    for i in 1..3 {
        let dxa = X[i] - X[i - 1];
        let dxb = X[i + 1] - X[i];
        let go = |j| g(&to, &vo, j);
        let gn = |j| g(&tn, &vn, j);
        let r1 = to[i - 1] * cv * (vo[i] + 2.0 * vo[i - 1]) / 12.0
            - to[i] * cv * (vo[i + 1] - vo[i - 1]) / 12.0
            - to[i + 1] * cv * (2.0 * vo[i + 1] + vo[i]) / 12.0
            + dxa * (go(i - 1) + 2.0 * go(i)) / 12.0
            + dxb * (2.0 * go(i) + go(i + 1)) / 12.0;
        let r2 = dxa * (gn(i - 1) + 2.0 * gn(i)) / 12.0 + dxb * (2.0 * gn(i) + gn(i + 1)) / 12.0;
        // Compare against the largest contributing term of each row.
        let big1 = (to[i] * cv * vo[i + 1]).abs().max((dxa * go(i)).abs());
        let big2 = (dxa * gn(i)).abs();
        assert!(
            (adj.rhs_level_one[i] - r1).abs() <= ADJUST_REL * big1,
            "rhs1 {i}"
        );
        assert!(
            (adj.rhs_level_two[i] - r2).abs() <= ADJUST_REL * big2,
            "rhs2 {i}"
        );
        close(
            adj.lower_level_two[i],
            -cv * (vn[i] + 2.0 * vn[i - 1]) / 12.0,
            ADJUST_REL,
            "lower2",
        );
        close(
            adj.diagonal_level_two[i],
            cv * (vn[i + 1] - vn[i - 1]) / 12.0,
            ADJUST_REL,
            "diag2",
        );
        close(
            adj.upper_level_two[i],
            cv * (2.0 * vn[i + 1] + vn[i]) / 12.0,
            ADJUST_REL,
            "upper2",
        );
    }
}

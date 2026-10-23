//! Public `FortranCompatible` step on prescribed-water transport, checked against
//! analytical references (never against solver output).
use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::solute::{advance_solute_step_with_processes, SoluteProcessInput};
use hydrus_core::{
    AcceptedWaterSnapshot, EquilibriumSorption, SoluteBackend, SoluteBoundaries,
    SoluteBoundaryMode as Boundary, SoluteConfig, SoluteSpecies, SoluteState, SoluteTransportInput,
    SoluteUnits, SoluteWorkspace,
};

const THETA: f64 = 0.43;
const RHO: f64 = 0.8;
const LENGTH: f64 = 2.0;

struct Run {
    x: Vec<f64>,
    conc: Vec<f64>,
}

/// Zero-flux inlet (`TotalFlux{0}`), `AdvectiveOutflow` outlet, q = 0.
#[allow(clippy::too_many_arguments)]
fn run(
    backend: SoluteBackend,
    nodes: usize,
    dispersion: f64,
    kd: &[f64],
    rates: &[f64],
    initial: &dyn Fn(usize, f64) -> f64,
    dt: f64,
    steps: usize,
) -> Run {
    run_flow(
        backend, nodes, dispersion, kd, rates, initial, dt, steps, 0.0, 0.0,
    )
}

/// Inlet `TotalFlux{q * c_in}`, `AdvectiveOutflow` outlet, uniform Darcy flux `q`.
#[allow(clippy::too_many_arguments)]
fn run_flow(
    backend: SoluteBackend,
    nodes: usize,
    dispersion: f64,
    kd: &[f64],
    rates: &[f64],
    initial: &dyn Fn(usize, f64) -> f64,
    dt: f64,
    steps: usize,
    q: f64,
    c_in: f64,
) -> Run {
    let n = kd.len();
    let x: Vec<f64> = (0..nodes)
        .map(|i| LENGTH * i as f64 / (nodes - 1) as f64)
        .collect();
    let mesh = Mesh1D::new(x.clone(), vec![0; nodes], None, 1, 0, 0.0).unwrap();
    let species = (0..n)
        .map(|s| SoluteSpecies {
            name: format!("S{s}"),
            decay_rate: rates[s],
            equilibrium_sorption: EquilibriumSorption::Linear {
                distribution_coefficient: kd[s],
            },
            kinetic_sorption: None,
        })
        .collect();
    let boundaries = (0..n)
        .map(|_| SoluteBoundaries {
            // q < 0 mirrors the problem: the inlet is the upper end and the flux
            // (positive into the domain) is -q * c_in.
            lower: if q < 0.0 {
                Boundary::AdvectiveOutflow
            } else {
                Boundary::TotalFlux { flux: q * c_in }
            },
            upper: if q < 0.0 {
                Boundary::TotalFlux { flux: -q * c_in }
            } else {
                Boundary::AdvectiveOutflow
            },
        })
        .collect();
    let config = SoluteConfig::new_with_species_boundaries(
        SoluteUnits {
            concentration: "C_ref".into(),
            sorbed_mass: "C_ref cm3/g".into(),
        },
        species,
        boundaries,
        1,
        &[],
    )
    .unwrap()
    .with_backend(backend);
    let init: Vec<f64> = (0..n)
        .flat_map(|s| x.iter().map(move |&xi| (s, xi)).collect::<Vec<_>>())
        .map(|(s, xi)| initial(s, xi))
        .collect();
    let mut state = SoluteState::new(&config, &mesh, 0.0, init, vec![0.0; n * nodes]).unwrap();
    let mut ws = SoluteWorkspace::new(nodes, n).unwrap();
    let theta = vec![THETA; nodes];
    let flux = vec![q; nodes + 1];
    let disp = vec![dispersion; nodes - 1];
    let density = vec![RHO; nodes];
    for step in 1..=steps {
        advance_solute_step_with_processes(
            &mut state,
            &mut ws,
            &mesh,
            &config,
            SoluteTransportInput {
                accepted_water: AcceptedWaterSnapshot {
                    accepted_time: step as f64 * dt,
                    water_contents: &theta,
                    face_fluxes: &flux,
                },
                start_water_contents: &theta,
                face_dispersion: &disp,
                timestep: dt,
                minimum_pivot: 1e-12,
            },
            Some(SoluteProcessInput {
                bulk_density: &density,
                max_iterations: 20,
                nonlinear_tolerance: 1e-6,
            }),
        )
        .unwrap();
    }
    Run {
        x,
        conc: state.concentrations().to_vec(),
    }
}

fn max_abs(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(p, q)| (p - q).abs())
        .fold(0.0, f64::max)
}

const BASE: f64 = 1.0;
const AMP: f64 = 0.25;
fn cosine(_: usize, x: f64) -> f64 {
    BASE + AMP * (std::f64::consts::PI * x / LENGTH).cos()
}

#[test]
fn public_fortran_compatible_diffusion_matches_cosine_decay() {
    // Exact: c0 + A exp(-D (pi/L)^2 t / R) cos(pi x/L), R = 1 + rho Kd / theta.
    // Tolerance 0.02 * A: 81 nodes, dt = 0.01 (second-order scheme, first-order
    // zero-gradient end rows); well above the f32 floor (~1e-6).
    let (d, dt, steps) = (0.2, 0.01, 100);
    for kd in [0.0, 0.5375] {
        let r = run(
            SoluteBackend::FortranCompatible,
            81,
            d,
            &[kd],
            &[0.0],
            &cosine,
            dt,
            steps,
        );
        let ret = 1.0 + RHO * kd / THETA;
        let k = std::f64::consts::PI / LENGTH;
        let decay = (-d * k * k * (dt * steps as f64) / ret).exp();
        let exact: Vec<f64> =
            r.x.iter()
                .map(|&x| BASE + AMP * decay * (k * x).cos())
                .collect();
        let err = max_abs(&r.conc, &exact);
        assert!(err < 0.02 * AMP, "kd={kd}: max error {err}");
        // The profile must really have moved (decay factor well below 1).
        assert!(decay < 0.95);
    }
}

#[test]
fn public_fortran_compatible_zero_order_decay_matches_analytical() {
    // reaction_zero: all rates 0, so every profile is constant in time; node 0 stays put.
    let init = |s: usize, _x: f64| [1.0, 0.25, 0.0][s];
    let r = run(
        SoluteBackend::FortranCompatible,
        5,
        0.0,
        &[0.5375, 0.0, 0.26875],
        &[0.0; 3],
        &init,
        1.0,
        4,
    );
    for s in 0..3 {
        for i in 0..5 {
            let want = [1.0, 0.25, 0.0][s];
            assert!(
                (r.conc[s * 5 + i] - want).abs() < 1e-5,
                "species {s} node {i}: {}",
                r.conc[s * 5 + i]
            );
        }
    }
}

#[test]
fn public_fortran_compatible_first_order_decay_matches_analytical() {
    // Uniform field, no transport gradient: c(t) = c0 exp(-k t) (decay acts on total mass,
    // so the equilibrium-sorbed case decays at the same rate). The scheme is first order
    // in dt (error halves with dt: 2.4e-3, 1.2e-3, 6e-4 at dt = 0.05, 0.025, 0.0125 for
    // t = 2); tolerance 1e-3 absolute on c0 = 1 at dt = 0.0125.
    let (k, dt, steps) = (0.3, 0.0125, 160);
    for kd in [0.0, 0.5375] {
        let r = run(
            SoluteBackend::FortranCompatible,
            9,
            0.2,
            &[kd],
            &[k],
            &|_, _| 1.0,
            dt,
            steps,
        );
        let exact = (-k * dt * steps as f64).exp();
        for c in &r.conc {
            assert!((c - exact).abs() < 1e-3, "kd={kd}: {c} vs {exact}");
        }
    }
}

#[test]
fn public_fortran_compatible_agrees_with_fvbe_on_resolved_diffusion() {
    // Loose stated tolerance 0.02 * A between the two backends on a resolved mesh.
    let go = |b| run(b, 81, 0.2, &[0.5375], &[0.0], &cosine, 0.01, 100);
    let fc = go(SoluteBackend::FortranCompatible);
    let fv = go(SoluteBackend::FiniteVolumeBackwardEuler);
    let diff = max_abs(&fc.conc, &fv.conc);
    assert!(diff < 0.02 * AMP, "backend difference {diff}");
}

// ---- chain production (22.3.8) ----

/// Bateman chain on TOTAL mass per volume (decay and production act on total mass):
/// M_j = theta R_j c_j, dM1/dt = -k1 M1, dMj/dt = k_{j-1} M_{j-1} - k_j M_j.
/// Returns the concentrations c_j(t) for a unit initial c_1 and zero daughters.
fn bateman(kd: &[f64], k: &[f64], t: f64) -> Vec<f64> {
    let ret: Vec<f64> = kd.iter().map(|d| 1.0 + RHO * d / THETA).collect();
    let m0 = THETA * ret[0]; // initial total mass of S1 for c1 = 1
    let (k1, k2, k3) = (k[0], k[1], k[2]);
    let m1 = m0 * (-k1 * t).exp();
    let (m2, m3);
    if (k1 - k2).abs() < 1e-12 && (k2 - k3).abs() < 1e-12 {
        m2 = m0 * k1 * t * (-k1 * t).exp();
        m3 = m0 * 0.5 * (k1 * t).powi(2) * (-k1 * t).exp();
    } else {
        assert!((k1 - k2).abs() > 1e-9 && (k1 - k3).abs() > 1e-9 && (k2 - k3).abs() > 1e-9);
        m2 = m0 * k1 / (k2 - k1) * ((-k1 * t).exp() - (-k2 * t).exp());
        m3 = m0
            * k1
            * k2
            * ((-k1 * t).exp() / ((k2 - k1) * (k3 - k1))
                + (-k2 * t).exp() / ((k1 - k2) * (k3 - k2))
                + (-k3 * t).exp() / ((k1 - k3) * (k2 - k3)));
    }
    vec![
        m1 / (THETA * ret[0]),
        m2 / (THETA * ret[1]),
        m3 / (THETA * ret[2]),
    ]
}

fn check_chain(kd: &[f64], rates: &[f64]) {
    // Uniform state, q = 0, D = 0: pure reaction. Scheme is first order in dt
    // (backward Euler on a stiff-free 0.1..0.4 /h system); dt = 0.001, t = 2:
    // tolerance 2e-3 of c0 (error ~ k^2 t dt / 2 ~ 4e-4 plus f32 floor).
    let (dt, steps) = (0.001, 2000);
    let r = run(
        SoluteBackend::FortranCompatible,
        9,
        0.0,
        kd,
        rates,
        &|s, _| if s == 0 { 1.0 } else { 0.0 },
        dt,
        steps,
    );
    let exact = bateman(kd, rates, dt * steps as f64);
    for (s, &want) in exact.iter().enumerate() {
        for (i, &c) in r.conc[s * 9..(s + 1) * 9].iter().enumerate() {
            assert!(
                (c - want).abs() < 2e-3,
                "species {s} node {i}: {c} vs {want}"
            );
        }
        assert!(want > 1e-2, "test must exercise species {s}");
    }
}

#[test]
fn public_fortran_compatible_three_species_chain_matches_bateman() {
    check_chain(&[0.5375, 0.0, 0.26875], &[0.2, 0.4, 0.1]);
}

#[test]
fn public_fortran_compatible_equal_rates_chain_matches_bateman() {
    check_chain(&[0.5375, 0.0, 0.26875], &[0.2, 0.2, 0.2]);
}

#[test]
fn public_fortran_compatible_chain_with_sorption_matches_analytical() {
    check_chain(&[0.5375, 0.8, 0.26875], &[0.2, 0.4, 0.1]);
    check_chain(&[0.5375, 0.8, 0.26875], &[0.3, 0.3, 0.3]);
}

// ---- advection-dispersion-decay with a flux inlet (22.3.8, scalar_robin) ----

const Q: f64 = 0.43; // v = q / theta = 1

/// Closed-form steady state of D c'' - v c' - R lambda c = 0 with the Robin inlet
/// v c - D c' = v c_in at s = 0 and c' = 0 at s = L.
fn robin_steady(kd: f64, lambda: f64, d: f64, c_in: f64) -> impl Fn(f64) -> f64 {
    let v = Q / THETA;
    let ret = 1.0 + RHO * kd / THETA;
    let disc = (v * v + 4.0 * d * ret * lambda).sqrt();
    let (r1, r2) = ((v + disc) / (2.0 * d), (v - disc) / (2.0 * d));
    // A r1 e^{r1 L} + B r2 e^{r2 L} = 0; v (A + B) - d (A r1 + B r2) = v c_in.
    let ratio = -r2 * (r2 * LENGTH).exp() / (r1 * (r1 * LENGTH).exp()); // A = ratio * B
    let b = v * c_in / (v * (ratio + 1.0) - d * (ratio * r1 + r2));
    let a = ratio * b;
    move |s| a * (r1 * s).exp() + b * (r2 * s).exp()
}

fn flow_run(nodes: usize, kd: f64, lambda: f64, d: f64, init: &dyn Fn(f64) -> f64, t: f64) -> Run {
    let dt = 0.01;
    run_flow(
        SoluteBackend::FortranCompatible,
        nodes,
        d,
        &[kd],
        &[lambda],
        &|_, x| init(x),
        dt,
        (t / dt).round() as usize,
        Q,
        1.0,
    )
}

#[test]

fn public_fortran_compatible_flux_inlet_uniform_state_stays_uniform() {
    // No decay: c = c_in everywhere is an exact solution for any Kd, D.
    for kd in [0.0, 0.5375] {
        let r = flow_run(65, kd, 0.0, 1.0, &|_| 1.0, 1.0);
        let worst = r.conc.iter().map(|c| (c - 1.0).abs()).fold(0.0, f64::max);
        assert!(worst < 1e-4, "kd {kd}: max deviation {worst}");
    }
}

#[test]

fn public_fortran_compatible_flux_inlet_steady_state_with_decay_holds() {
    // scalar_robin parameters: v = 1, D = 1, R = 1.5, lambda = 0.2, L = 2.
    // Starting from the closed-form steady state the solution must stay there
    // (tolerance 0.02 c_in: second-order space on 65 nodes plus first-order end rows).
    let steady = robin_steady(0.26875, 0.2, 1.0, 1.0);
    for nodes in [33, 65] {
        let r = flow_run(nodes, 0.26875, 0.2, 1.0, &steady, 1.0);
        let exact: Vec<f64> = r.x.iter().map(|&s| steady(s)).collect();
        let err = max_abs(&r.conc, &exact);
        assert!(err < 0.02, "{nodes} nodes: max deviation {err}");
        assert!(r.conc.iter().all(|&c| c <= 1.0 + 1e-6), "c exceeds c_in");
    }
}

#[test]

fn public_fortran_compatible_flux_inlet_transient_relaxes_to_steady_state() {
    // From c = 0 toward the closed-form steady state: bounded by c_in and by the
    // steady profile (maximum principle for the monotone start), and at t = 30
    // equal to it within 0.02 c_in.
    let steady = robin_steady(0.26875, 0.2, 1.0, 1.0);
    let early = flow_run(65, 0.26875, 0.2, 1.0, &|_| 0.0, 1.0);
    for (i, &c) in early.conc.iter().enumerate() {
        assert!(
            c >= -1e-6 && c <= steady(early.x[i]) + 0.02,
            "node {i}: {c} vs steady {}",
            steady(early.x[i])
        );
    }
    let late = flow_run(65, 0.26875, 0.2, 1.0, &|_| 0.0, 30.0);
    let exact: Vec<f64> = late.x.iter().map(|&s| steady(s)).collect();
    let err = max_abs(&late.conc, &exact);
    assert!(err < 0.02, "late max deviation {err}");
}

#[test]
fn public_fortran_compatible_flux_inlet_mirrored_flow_matches_forward_flow() {
    // q < 0 with the inlet at the upper end is the mirror image of q > 0: the
    // analytic steady state is steady(L - s). Uniform state held, steady state held.
    let steady = robin_steady(0.26875, 0.2, 1.0, 1.0);
    let dt = 0.01;
    let mirror = |f: &dyn Fn(f64) -> f64, kd: f64, lambda: f64| {
        run_flow(
            SoluteBackend::FortranCompatible,
            65,
            1.0,
            &[kd],
            &[lambda],
            &|_, x| f(x),
            dt,
            100,
            -Q,
            1.0,
        )
    };
    let uniform = mirror(&|_| 1.0, 0.5375, 0.0);
    assert!(uniform.conc.iter().all(|c| (c - 1.0).abs() < 1e-4));
    let r = mirror(&|x| steady(LENGTH - x), 0.26875, 0.2);
    let exact: Vec<f64> = r.x.iter().map(|&x| steady(LENGTH - x)).collect();
    let err = max_abs(&r.conc, &exact);
    assert!(err < 0.02, "mirrored max deviation {err}");
}

#[test]
fn public_fortran_compatible_forward_and_mirrored_flow_agree() {
    let fwd = flow_run(65, 0.26875, 0.2, 1.0, &|_| 0.0, 1.0);
    let back = run_flow(
        SoluteBackend::FortranCompatible,
        65,
        1.0,
        &[0.26875],
        &[0.2],
        &|_, _| 0.0,
        0.01,
        100,
        -Q,
        1.0,
    );
    let rev: Vec<f64> = back.conc.iter().rev().cloned().collect();
    // the two orientations run the same kernel problem (mirrored): agree to f32 floor
    assert!(max_abs(&fwd.conc, &rev) < 1e-5);
}

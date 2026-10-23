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
            lower: Boundary::TotalFlux { flux: 0.0 },
            upper: Boundary::AdvectiveOutflow,
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
    let flux = vec![0.0; nodes + 1];
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

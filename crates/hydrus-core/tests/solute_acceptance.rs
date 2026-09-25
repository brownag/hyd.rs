//! M5 balance contract fixtures. Inputs are deterministic in-memory cases;
//! manual Chapters 11.3–11.5 are provenance candidates, but their decks and
//! expected outputs are unavailable. These are invariant checks, not calibrated
//! trajectories or Fortran parity results.

use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::solute::{advance_solute_step_with_processes, SoluteProcessInput};
use hydrus_core::{
    AcceptedWaterSnapshot, DeferredSoluteFeature, EquilibriumSorption, KineticSorption,
    SoluteBoundaries, SoluteBoundaryMode as Boundary, SoluteConfig, SoluteConfigError,
    SoluteSpecies, SoluteState, SoluteStepError, SoluteTransportInput, SoluteUnits,
    SoluteWorkspace,
};

fn mesh() -> Mesh1D {
    Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap()
}

fn species(
    name: &str,
    decay_rate: f64,
    equilibrium_sorption: EquilibriumSorption,
    kinetic_sorption: Option<KineticSorption>,
) -> SoluteSpecies {
    SoluteSpecies {
        name: name.into(),
        decay_rate,
        equilibrium_sorption,
        kinetic_sorption,
    }
}

fn config(species: Vec<SoluteSpecies>, lower: Boundary, upper: Boundary) -> SoluteConfig {
    SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        species,
        SoluteBoundaries { lower, upper },
        1,
        &[],
    )
    .unwrap()
}

fn input<'a>(
    time: f64,
    theta: &'a [f64],
    faces: &'a [f64],
    dispersion: &'a [f64],
    timestep: f64,
) -> SoluteTransportInput<'a> {
    input_with_start(time, theta, theta, faces, dispersion, timestep)
}

fn input_with_start<'a>(
    time: f64,
    start_theta: &'a [f64],
    end_theta: &'a [f64],
    faces: &'a [f64],
    dispersion: &'a [f64],
    timestep: f64,
) -> SoluteTransportInput<'a> {
    SoluteTransportInput {
        accepted_water: AcceptedWaterSnapshot {
            accepted_time: time,
            water_contents: end_theta,
            face_fluxes: faces,
        },
        start_water_contents: start_theta,
        face_dispersion: dispersion,
        timestep,
        minimum_pivot: 1e-12,
    }
}

fn run(
    state: &mut SoluteState,
    config: &SoluteConfig,
    theta: &[f64; 3],
    faces: &[f64; 4],
    dispersion: &[f64; 2],
    timestep: f64,
    density: Option<&[f64; 3]>,
) -> Result<(), SoluteStepError> {
    let mesh = mesh();
    let mut workspace = SoluteWorkspace::new(3, config.species().len()).unwrap();
    let accepted_time = state.time() + timestep;
    advance_solute_step_with_processes(
        state,
        &mut workspace,
        &mesh,
        config,
        input(accepted_time, theta, faces, dispersion, timestep),
        density.map(|bulk_density| SoluteProcessInput {
            bulk_density,
            max_iterations: 30,
            nonlinear_tolerance: 1e-10,
        }),
    )
}

fn close(left: f64, right: f64, tolerance: f64) {
    assert!((left - right).abs() <= tolerance, "{left} != {right}");
}

#[test]
fn conservative_transport_reports_signed_total_boundary_mass_and_closure() {
    // Independent in-memory finite-volume invariant case; fluxes are caller
    // inputs, and no manual or Fortran output is used as an expected result.
    let config = config(
        vec![species("tracer", 0.0, EquilibriumSorption::None, None)],
        Boundary::TotalFlux { flux: 0.4 },
        Boundary::TotalFlux { flux: -0.1 },
    );
    let theta = [0.3; 3];
    let mut state =
        SoluteState::new(&config, &mesh(), 0.0, vec![1.0, 0.2, 0.1], vec![0.0; 3]).unwrap();
    let timestep = 0.2;
    run(
        &mut state, &config, &theta, &[0.0; 4], &[0.1; 2], timestep, None,
    )
    .unwrap();

    let balance = state.accounting().last_step()[0];
    close(balance.boundary_exchange, (0.4 - 0.1) * timestep, 1e-12);
    close(balance.storage_change, balance.boundary_exchange, 1e-12);
    assert!(balance.absolute_residual <= 1e-12);
    close(
        state.accounting().cumulative_boundary_mass()[0],
        balance.boundary_exchange,
        1e-12,
    );
    assert_eq!(state.accounting().accepted_steps(), 1);
    assert_eq!(state.time(), timestep);
}

#[test]
fn changing_water_content_uses_both_interval_endpoints_in_storage_balance() {
    let config = config(
        vec![species("tracer", 0.0, EquilibriumSorption::None, None)],
        Boundary::TotalFlux { flux: 0.0 },
        Boundary::TotalFlux { flux: 0.0 },
    );
    let mesh = mesh();
    let mut state = SoluteState::new(&config, &mesh, 0.0, vec![2.0; 3], vec![0.0; 3]).unwrap();
    let mut workspace = SoluteWorkspace::new(3, 1).unwrap();
    advance_solute_step_with_processes(
        &mut state,
        &mut workspace,
        &mesh,
        &config,
        input_with_start(0.1, &[0.2; 3], &[0.3; 3], &[0.0; 4], &[0.0; 2], 0.1),
        None,
    )
    .unwrap();

    // Control-volume widths sum to 2; (0.3 - 0.2) * C(2) * 2 = 0.4.
    let balance = state.accounting().last_step()[0];
    close(balance.storage_change, 0.4, 1e-12);
    close(balance.boundary_exchange, 0.0, 1e-12);
    close(balance.absolute_residual, 0.4, 1e-12);
}

#[test]
fn reaction_and_both_sorbed_pools_are_included_in_per_species_balance() {
    // Source-anchored processes use compatible declared units: C [mg/L], S
    // [mg/kg], rho_b [kg/L], so rho_b*S and theta*C are both [mg/L].
    let config = config(
        vec![
            species(
                "parent",
                0.3,
                EquilibriumSorption::Linear {
                    distribution_coefficient: 0.8,
                },
                Some(KineticSorption {
                    capacity_coefficient: 0.4,
                    transfer_rate: 0.5,
                }),
            ),
            species("daughter", 0.0, EquilibriumSorption::None, None),
        ],
        Boundary::TotalFlux { flux: 0.0 },
        Boundary::TotalFlux { flux: 0.0 },
    );
    let theta = [0.3; 3];
    let density = [1.1; 3];
    let mut state = SoluteState::new(
        &config,
        &mesh(),
        0.0,
        vec![1.0, 0.8, 0.4, 0.0, 0.0, 0.0],
        vec![0.2, 0.1, 0.05, 0.0, 0.0, 0.0],
    )
    .unwrap();
    run(
        &mut state,
        &config,
        &theta,
        &[0.0; 4],
        &[0.0; 2],
        0.1,
        Some(&density),
    )
    .unwrap();

    for (index, balance) in state.accounting().last_step().iter().enumerate() {
        close(
            balance.storage_change,
            balance.boundary_exchange + balance.reaction_change,
            1e-10,
        );
        assert!(balance.absolute_residual <= 1e-10);
        close(
            state.accounting().cumulative_reaction_mass()[index],
            balance.reaction_change,
            1e-12,
        );
    }
    assert!(state.kinetic_sorbed()[0] > 0.2);
    assert!(state.concentrations()[3] > 0.0);
}

#[test]
fn concentration_boundary_exchange_is_explicitly_an_internal_closure_inference() {
    // An imposed concentration row has no independently observed solute-flux
    // input. Its net exchange is inferred from discrete storage and reaction.
    let config = config(
        vec![species("tracer", 0.0, EquilibriumSorption::None, None)],
        Boundary::Concentration { value: 1.5 },
        Boundary::TotalFlux { flux: 0.0 },
    );
    let theta = [0.3; 3];
    let mut state = SoluteState::new(&config, &mesh(), 0.0, vec![0.2; 3], vec![0.0; 3]).unwrap();
    run(&mut state, &config, &theta, &[0.0; 4], &[0.1; 2], 0.1, None).unwrap();
    let balance = state.accounting().last_step()[0];
    close(balance.boundary_exchange, balance.storage_change, 1e-12);
    assert!(balance.boundary_exchange > 0.0);
    assert!(balance.absolute_residual <= 1e-12);
}

#[test]
fn mismatched_accepted_time_rejects_before_mutation_and_unsupported_modes_are_typed() {
    let config = config(
        vec![species("tracer", 0.0, EquilibriumSorption::None, None)],
        Boundary::TotalFlux { flux: 0.0 },
        Boundary::TotalFlux { flux: 0.0 },
    );
    let mesh = mesh();
    let mut state = SoluteState::new(&config, &mesh, 1.0, vec![0.2; 3], vec![0.0; 3]).unwrap();
    let before = state.clone();
    let mut workspace = SoluteWorkspace::new(3, 1).unwrap();
    let error = advance_solute_step_with_processes(
        &mut state,
        &mut workspace,
        &mesh,
        &config,
        input(1.3, &[0.3; 3], &[0.0; 4], &[0.0; 2], 0.2),
        None,
    )
    .unwrap_err();
    assert!(matches!(error, SoluteStepError::Configuration(_)));
    assert_eq!(state, before);

    let error = advance_solute_step_with_processes(
        &mut state,
        &mut workspace,
        &mesh,
        &config,
        input(1.2, &[0.3; 2], &[0.0; 4], &[0.0; 2], 0.2),
        None,
    )
    .unwrap_err();
    assert!(matches!(error, SoluteStepError::Configuration(_)));
    assert_eq!(state, before);

    let unsupported = SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![species("tracer", 0.0, EquilibriumSorption::None, None)],
        SoluteBoundaries {
            lower: Boundary::TotalFlux { flux: 0.0 },
            upper: Boundary::TotalFlux { flux: 0.0 },
        },
        1,
        &[DeferredSoluteFeature::MobileImmobileTransport],
    );
    assert!(matches!(
        unsupported,
        Err(SoluteConfigError::Unsupported { .. })
    ));
}

#[test]
fn malformed_start_water_contents_reject_before_accepted_state_mutation() {
    let config = config(
        vec![species("tracer", 0.0, EquilibriumSorption::None, None)],
        Boundary::TotalFlux { flux: 0.0 },
        Boundary::TotalFlux { flux: 0.0 },
    );
    let mesh = mesh();
    let mut state = SoluteState::new(&config, &mesh, 1.0, vec![0.2; 3], vec![0.0; 3]).unwrap();
    let before = state.clone();
    let mut workspace = SoluteWorkspace::new(3, 1).unwrap();

    for start_theta in [
        &[0.3, 0.3][..],
        &[0.3, f64::NAN, 0.3][..],
        &[0.3, 0.0, 0.3][..],
    ] {
        let error = advance_solute_step_with_processes(
            &mut state,
            &mut workspace,
            &mesh,
            &config,
            input_with_start(1.1, start_theta, &[0.3; 3], &[0.0; 4], &[0.0; 2], 0.1),
            None,
        )
        .unwrap_err();
        assert!(matches!(error, SoluteStepError::Configuration(_)));
        assert_eq!(state, before);
    }
}

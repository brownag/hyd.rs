use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::solute::{advance_solute_step_with_processes, SoluteProcessInput};
use hydrus_core::{
    AcceptedWaterSnapshot, EquilibriumSorption, SoluteBoundaries, SoluteBoundaryMode as Boundary,
    SoluteConfig, SoluteSpecies, SoluteState, SoluteTransportInput, SoluteUnits, SoluteWorkspace,
};

fn mesh() -> Mesh1D {
    Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap()
}

fn species(name: &str, decay_rate: f64) -> SoluteSpecies {
    SoluteSpecies {
        name: name.into(),
        decay_rate,
        equilibrium_sorption: EquilibriumSorption::None,
        kinetic_sorption: None,
    }
}

fn pair(lower: Boundary, upper: Boundary) -> SoluteBoundaries {
    SoluteBoundaries { lower, upper }
}

fn config(species: Vec<SoluteSpecies>, boundaries: Vec<SoluteBoundaries>) -> SoluteConfig {
    SoluteConfig::new_with_species_boundaries(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        species,
        boundaries,
        1,
        &[],
    )
    .unwrap()
}

fn input<'a>(
    time: f64,
    theta: &'a [f64; 3],
    faces: &'a [f64; 4],
    dispersion: &'a [f64; 2],
    timestep: f64,
) -> SoluteTransportInput<'a> {
    SoluteTransportInput {
        accepted_water: AcceptedWaterSnapshot {
            accepted_time: time,
            water_contents: theta,
            face_fluxes: faces,
        },
        start_water_contents: theta,
        face_dispersion: dispersion,
        timestep,
        minimum_pivot: 1e-12,
    }
}

fn run(
    state: &mut SoluteState,
    config: &SoluteConfig,
    timestep: f64,
    process: bool,
) -> Result<(), hydrus_core::SoluteStepError> {
    let mesh = mesh();
    let mut workspace = SoluteWorkspace::new(3, config.species().len()).unwrap();
    let theta = [0.3; 3];
    let faces = [0.0; 4];
    let dispersion = [0.05; 2];
    advance_solute_step_with_processes(
        state,
        &mut workspace,
        &mesh,
        config,
        input(
            state.time() + timestep,
            &theta,
            &faces,
            &dispersion,
            timestep,
        ),
        if process {
            Some(SoluteProcessInput {
                bulk_density: &[1.0; 3],
                max_iterations: 30,
                nonlinear_tolerance: 1e-10,
            })
        } else {
            None
        },
    )
}

fn inventory(state: &SoluteState, theta: &[f64; 3], species: usize) -> f64 {
    let widths = [0.5, 1.0, 0.5];
    let offset = species * 3;
    (0..3)
        .map(|node| widths[node] * theta[node] * state.concentrations()[offset + node])
        .sum()
}

fn close(left: f64, right: f64, tolerance: f64) {
    assert!((left - right).abs() <= tolerance, "{left} != {right}");
}

#[test]
fn species_flux_pairs_have_independent_signed_accounting_and_inventory() {
    for process in [false, true] {
        let config = config(
            vec![species("A", 0.0), species("B", 0.0)],
            vec![
                pair(
                    Boundary::TotalFlux { flux: 0.4 },
                    Boundary::TotalFlux { flux: -0.1 },
                ),
                pair(
                    Boundary::TotalFlux { flux: 0.05 },
                    Boundary::TotalFlux { flux: -0.02 },
                ),
            ],
        );
        let theta = [0.3; 3];
        let mut state = SoluteState::new(
            &config,
            &mesh(),
            0.0,
            vec![1.0, 0.8, 0.5, 0.6, 0.4, 0.2],
            vec![0.0; 6],
        )
        .unwrap();
        let before = [inventory(&state, &theta, 0), inventory(&state, &theta, 1)];
        let timestep = 0.2;
        run(&mut state, &config, timestep, process).unwrap();
        let after = [inventory(&state, &theta, 0), inventory(&state, &theta, 1)];
        close(after[0] - before[0], (0.4 - 0.1) * timestep, 1e-11);
        close(after[1] - before[1], (0.05 - 0.02) * timestep, 1e-11);
        close(
            state.accounting().last_step()[0].boundary_exchange,
            0.06,
            1e-12,
        );
        close(
            state.accounting().last_step()[1].boundary_exchange,
            0.006,
            1e-12,
        );
        for balance in state.accounting().last_step() {
            close(balance.storage_change, balance.boundary_exchange, 1e-11);
            assert!(balance.absolute_residual < 1e-11);
        }
    }
}

#[test]
fn broadcast_and_repeated_explicit_pairs_produce_identical_steps() {
    let species = vec![species("A", 0.1), species("B", 0.0)];
    let shared = pair(
        Boundary::TotalFlux { flux: 0.2 },
        Boundary::TotalFlux { flux: -0.05 },
    );
    let broadcast = SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        species.clone(),
        shared,
        1,
        &[],
    )
    .unwrap();
    let explicit = config(species, vec![shared, shared]);
    let mut broadcast_state =
        SoluteState::new(&broadcast, &mesh(), 0.0, vec![1.0; 6], vec![0.0; 6]).unwrap();
    let mut explicit_state = broadcast_state.clone();
    run(&mut broadcast_state, &broadcast, 0.1, true).unwrap();
    run(&mut explicit_state, &explicit, 0.1, true).unwrap();
    assert_eq!(broadcast_state, explicit_state);
}

#[test]
fn mixed_endpoint_modes_keep_flux_species_accounting_independent() {
    for process in [false, true] {
        let config = config(
            vec![
                species("concentration-A", 0.0),
                species("concentration-B", 0.0),
                species("flux", 0.0),
            ],
            vec![
                pair(
                    Boundary::Concentration { value: 1.5 },
                    Boundary::TotalFlux { flux: 0.0 },
                ),
                pair(
                    Boundary::Concentration { value: 0.8 },
                    Boundary::TotalFlux { flux: 0.0 },
                ),
                pair(
                    Boundary::TotalFlux { flux: 0.3 },
                    Boundary::TotalFlux { flux: -0.05 },
                ),
            ],
        );
        let mut state =
            SoluteState::new(&config, &mesh(), 0.0, vec![0.5; 9], vec![0.0; 9]).unwrap();
        run(&mut state, &config, 0.1, process).unwrap();
        close(state.concentrations()[0], 1.5, 1e-12);
        close(state.concentrations()[3], 0.8, 1e-12);
        close(
            state.accounting().last_step()[2].boundary_exchange,
            0.025,
            1e-12,
        );
        close(
            state.accounting().last_step()[2].storage_change,
            0.025,
            1e-12,
        );
        assert!(state.accounting().last_step()[0].boundary_exchange.abs() > 1e-3);
        assert!(state.accounting().last_step()[1].boundary_exchange.abs() > 1e-3);
    }
}

#[test]
fn coupled_chain_receives_direct_flux_only_for_parent_and_conserves_total_reaction_mass() {
    let config = config(
        vec![
            species("NH4", 0.2),
            species("NO2", 0.15),
            species("NO3", 0.0),
        ],
        vec![
            pair(
                Boundary::TotalFlux { flux: 0.1 },
                Boundary::TotalFlux { flux: 0.0 },
            ),
            pair(
                Boundary::TotalFlux { flux: 0.0 },
                Boundary::TotalFlux { flux: 0.0 },
            ),
            pair(
                Boundary::TotalFlux { flux: 0.0 },
                Boundary::TotalFlux { flux: 0.0 },
            ),
        ],
    );
    let theta = [0.3; 3];
    let initial = vec![1.0, 0.8, 0.6, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let mut state = SoluteState::new(&config, &mesh(), 0.0, initial, vec![0.0; 9]).unwrap();
    let before: f64 = (0..3).map(|i| inventory(&state, &theta, i)).sum();
    let timestep = 0.1;
    run(&mut state, &config, timestep, true).unwrap();
    let after: f64 = (0..3).map(|i| inventory(&state, &theta, i)).sum();
    let reaction_total: f64 = state
        .accounting()
        .last_step()
        .iter()
        .map(|balance| balance.reaction_change)
        .sum();
    close(
        state.accounting().last_step()[0].boundary_exchange,
        0.01,
        1e-12,
    );
    close(
        state.accounting().last_step()[1].boundary_exchange,
        0.0,
        1e-12,
    );
    close(
        state.accounting().last_step()[2].boundary_exchange,
        0.0,
        1e-12,
    );
    // Parent loss is transferred through both daughters; the terminal species
    // has zero decay, so the chain's independently summed reaction change is 0.
    close(reaction_total, 0.0, 1e-10);
    close(after - before, 0.1 * timestep, 1e-10);
    assert!(inventory(&state, &theta, 1) > 0.0);
    assert!(inventory(&state, &theta, 2) > 0.0);
    for balance in state.accounting().last_step() {
        close(
            balance.storage_change,
            balance.boundary_exchange + balance.reaction_change,
            1e-10,
        );
    }
}

#[test]
fn later_species_failure_rolls_back_earlier_trial_and_accounting() {
    let original_config = config(
        vec![species("parent", 0.1), species("daughter", 0.0)],
        vec![
            pair(
                Boundary::TotalFlux { flux: 0.2 },
                Boundary::TotalFlux { flux: 0.0 },
            ),
            pair(
                Boundary::Concentration { value: 0.0 },
                Boundary::TotalFlux { flux: 0.0 },
            ),
        ],
    );
    let mut state = SoluteState::new(
        &original_config,
        &mesh(),
        0.0,
        vec![1.0, 0.5, 0.2, 0.0, 0.0, 0.0],
        vec![0.0; 6],
    )
    .unwrap();
    let accepted = state.clone();
    // The zero daughter concentration with exponent < 1 fails inside its solve,
    // after the parent has already been solved into the workspace trial.
    let mut failing_species = original_config.species().to_vec();
    failing_species[1].equilibrium_sorption = EquilibriumSorption::Nonlinear {
        fraction: 1.0,
        coefficient: 1.0,
        saturation: 0.0,
        exponent: 0.5,
    };
    let failing = config(failing_species, original_config.boundaries().to_vec());
    assert!(run(&mut state, &failing, 0.1, true).is_err());
    assert_eq!(state, accepted);
}

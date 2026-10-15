use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::solute::{advance_solute_step_with_processes, SoluteProcessInput};
use hydrus_core::{
    AcceptedWaterSnapshot, EquilibriumSorption, KineticSorption, SoluteBackend, SoluteBoundaries,
    SoluteBoundaryMode as Mode, SoluteConfig, SoluteEndpoint, SoluteSpecies, SoluteState,
    SoluteStepError, SoluteTransportInput, SoluteUnits, SoluteWorkspace,
};

fn species(decay: f64, sorption: EquilibriumSorption) -> SoluteSpecies {
    SoluteSpecies {
        name: "tracer".into(),
        decay_rate: decay,
        equilibrium_sorption: sorption,
        kinetic_sorption: None,
    }
}

fn named_species(name: &str, decay: f64) -> SoluteSpecies {
    SoluteSpecies {
        name: name.into(),
        decay_rate: decay,
        equilibrium_sorption: EquilibriumSorption::None,
        kinetic_sorption: None,
    }
}

fn config(lower: Mode, upper: Mode, decay: f64, sorption: EquilibriumSorption) -> SoluteConfig {
    SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![species(decay, sorption)],
        SoluteBoundaries { lower, upper },
        1,
        &[],
    )
    .unwrap()
    .with_backend(SoluteBackend::FiniteVolumeBackwardEuler)
}

fn mesh(coords: Vec<f64>) -> Mesh1D {
    let count = coords.len();
    Mesh1D::new(coords, vec![0; count], None, 1, 0, 0.0).unwrap()
}

fn advance(
    state: &mut SoluteState,
    mesh: &Mesh1D,
    config: &SoluteConfig,
    theta: &[f64],
    faces: &[f64],
    dispersion: &[f64],
    process: bool,
) -> Result<(), SoluteStepError> {
    let mut workspace = SoluteWorkspace::new(theta.len(), config.species().len()).unwrap();
    let input = SoluteTransportInput {
        accepted_water: AcceptedWaterSnapshot {
            accepted_time: state.time() + 1.0,
            water_contents: theta,
            face_fluxes: faces,
        },
        start_water_contents: theta,
        face_dispersion: dispersion,
        timestep: 1.0,
        minimum_pivot: 1e-12,
    };
    let density = vec![1.0; theta.len()];
    advance_solute_step_with_processes(
        state,
        &mut workspace,
        mesh,
        config,
        input,
        process.then_some(SoluteProcessInput {
            bulk_density: &density,
            max_iterations: 10,
            nonlinear_tolerance: 1e-12,
        }),
    )
}

fn close(actual: f64, expected: f64) {
    let scale = actual.abs().max(expected.abs());
    let bound = 64.0 * f64::EPSILON * scale.max(1.0);
    assert!(
        (actual - expected).abs() <= bound,
        "{actual} != {expected} (bound {bound})"
    );
}

#[test]
fn lower_and_upper_outlets_match_independent_nonuniform_finite_volume_vectors_in_both_paths() {
    let mesh = mesh(vec![0.0, 2.0, 4.0]);
    let cases = [
        (
            Mode::AdvectiveOutflow,
            Mode::TotalFlux { flux: 0.0 },
            vec![-0.5, -0.5, -0.5, 0.0],
            vec![1.75, 0.5625, 0.375],
            vec![1.0, 0.5, 0.25],
            -0.5,
        ),
        (
            Mode::TotalFlux { flux: 0.0 },
            Mode::AdvectiveOutflow,
            vec![0.0, 0.5, 0.5, 0.5],
            vec![0.375, 0.5625, 1.75],
            vec![0.25, 0.5, 1.0],
            -0.5,
        ),
    ];
    for (lower, upper, faces, old, expected, exchange) in cases {
        for process in [false, true] {
            let config = config(lower, upper, 0.0, EquilibriumSorption::None);
            let mut state =
                SoluteState::new(&config, &mesh, 0.0, old.clone(), vec![0.0; 3]).unwrap();
            advance(
                &mut state, &mesh, &config, &[0.5; 3], &faces, &[1.0; 2], process,
            )
            .unwrap();
            for (actual, expected) in state.concentrations().iter().zip(&expected) {
                close(*actual, *expected);
            }
            let widths = [1.0, 2.0, 1.0];
            let old_inventory = 0.5
                * old
                    .iter()
                    .zip(widths)
                    .map(|(concentration, width)| concentration * width)
                    .sum::<f64>();
            let new_inventory = 0.5
                * state
                    .concentrations()
                    .iter()
                    .zip(widths)
                    .map(|(concentration, width)| concentration * width)
                    .sum::<f64>();
            let independent_outlet = if matches!(lower, Mode::AdvectiveOutflow) {
                faces[0] * state.concentrations()[0]
            } else {
                -faces[3] * state.concentrations()[2]
            };
            close(new_inventory - old_inventory, independent_outlet);
            close(
                state.accounting().last_step()[0].boundary_exchange,
                independent_outlet,
            );
            close(new_inventory - old_inventory, exchange);
            let scale = new_inventory.abs() + old_inventory.abs() + independent_outlet.abs();
            assert!(
                (new_inventory - old_inventory - independent_outlet).abs()
                    <= 64.0 * f64::EPSILON * scale.max(1.0)
            );
        }
    }
}

#[test]
fn zero_flow_outlets_preserve_concentration_and_have_zero_exchange() {
    let mesh = mesh(vec![0.0, 2.0, 4.0]);
    let config = config(
        Mode::AdvectiveOutflow,
        Mode::AdvectiveOutflow,
        0.0,
        EquilibriumSorption::None,
    );
    let initial = vec![2.0, 0.5, 0.75];
    for process in [false, true] {
        let mut state =
            SoluteState::new(&config, &mesh, 0.0, initial.clone(), vec![0.0; 3]).unwrap();
        advance(
            &mut state, &mesh, &config, &[0.5; 3], &[0.0; 4], &[0.0; 2], process,
        )
        .unwrap();
        assert_eq!(state.concentrations(), initial);
        let widths = [1.0, 2.0, 1.0];
        let old_inventory = 0.5 * initial.iter().zip(widths).map(|(c, w)| c * w).sum::<f64>();
        let new_inventory = 0.5
            * state
                .concentrations()
                .iter()
                .zip(widths)
                .map(|(c, w)| c * w)
                .sum::<f64>();
        close(new_inventory, old_inventory);
        close(state.accounting().last_step()[0].boundary_exchange, 0.0);
    }
}

#[test]
fn process_outlet_preserves_sorbed_endpoint_storage_and_accounts_reaction_independently() {
    let mesh = mesh(vec![0.0, 2.0]);
    let config = config(
        Mode::AdvectiveOutflow,
        Mode::TotalFlux { flux: 0.0 },
        0.5,
        EquilibriumSorption::Linear {
            distribution_coefficient: 1.0,
        },
    );
    let mut state =
        SoluteState::new(&config, &mesh, 0.0, vec![11.0 / 3.0, 1.5], vec![0.0; 2]).unwrap();
    advance(
        &mut state,
        &mesh,
        &config,
        &[0.5; 2],
        &[-0.5, 0.0, 0.0],
        &[0.0],
        true,
    )
    .unwrap();
    close(state.concentrations()[0], 2.0);
    close(state.concentrations()[1], 1.0);
    let balance = state.accounting().last_step()[0];
    let old_inventory = 1.5 * (11.0 / 3.0 + 1.5);
    let new_inventory = 1.5 * (state.concentrations()[0] + state.concentrations()[1]);
    close(new_inventory - old_inventory, -3.25);
    close(balance.boundary_exchange, -1.0);
    close(balance.reaction_change, -2.25);
    close(balance.storage_change, -3.25);
}

#[test]
fn inward_flow_is_typed_and_rejected_before_state_changes_for_either_endpoint() {
    let mesh = mesh(vec![0.0, 1.0]);
    for (lower, upper, faces, endpoint, flux) in [
        (
            Mode::AdvectiveOutflow,
            Mode::TotalFlux { flux: 0.0 },
            vec![0.25, 0.0, 0.0],
            SoluteEndpoint::Lower,
            0.25,
        ),
        (
            Mode::TotalFlux { flux: 0.0 },
            Mode::AdvectiveOutflow,
            vec![0.0, 0.0, -0.25],
            SoluteEndpoint::Upper,
            -0.25,
        ),
    ] {
        let config = config(lower, upper, 0.0, EquilibriumSorption::None);
        let mut state =
            SoluteState::new(&config, &mesh, 0.0, vec![1.0, 0.5], vec![0.0; 2]).unwrap();
        let original = state.clone();
        let result = advance(&mut state, &mesh, &config, &[0.5; 2], &faces, &[0.0], false);
        assert!(
            matches!(result, Err(SoluteStepError::InwardFlowAtOutlet { species: 0, endpoint: actual, flux: actual_flux }) if actual == endpoint && actual_flux == flux)
        );
        assert_eq!(state, original);
    }
}

#[test]
fn later_species_reversal_and_nonfinite_water_flux_preserve_all_accepted_state() {
    let mesh = mesh(vec![0.0, 1.0]);
    let boundaries = vec![
        SoluteBoundaries {
            lower: Mode::TotalFlux { flux: 0.0 },
            upper: Mode::TotalFlux { flux: 0.0 },
        },
        SoluteBoundaries {
            lower: Mode::AdvectiveOutflow,
            upper: Mode::TotalFlux { flux: 0.0 },
        },
    ];
    let later_config = SoluteConfig::new_with_species_boundaries(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![named_species("first", 0.0), named_species("second", 0.0)],
        boundaries,
        1,
        &[],
    )
    .unwrap()
    .with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    let mut state = SoluteState::new(
        &later_config,
        &mesh,
        0.0,
        vec![1.0, 0.5, 0.25, 0.75],
        vec![0.0; 4],
    )
    .unwrap();
    let original = state.clone();
    let result = advance(
        &mut state,
        &mesh,
        &later_config,
        &[0.5; 2],
        &[0.25, 0.0, 0.0],
        &[0.0],
        false,
    );
    assert!(matches!(
        result,
        Err(SoluteStepError::InwardFlowAtOutlet {
            species: 1,
            endpoint: SoluteEndpoint::Lower,
            flux: 0.25
        })
    ));
    assert_eq!(state, original);

    let single = config(
        Mode::AdvectiveOutflow,
        Mode::TotalFlux { flux: 0.0 },
        0.0,
        EquilibriumSorption::None,
    );
    let mut state = SoluteState::new(&single, &mesh, 0.0, vec![1.0, 0.5], vec![0.0; 2]).unwrap();
    let original = state.clone();
    let result = advance(
        &mut state,
        &mesh,
        &single,
        &[0.5; 2],
        &[f64::NAN, 0.0, 0.0],
        &[0.0],
        false,
    );
    assert!(matches!(result, Err(SoluteStepError::Configuration(_))));
    assert_eq!(state, original);
    let result = advance(
        &mut state,
        &mesh,
        &single,
        &[0.5; 2],
        &[0.0; 2],
        &[0.0],
        false,
    );
    assert!(matches!(result, Err(SoluteStepError::Configuration(_))));
    assert_eq!(state, original);
}

#[test]
fn later_species_process_failure_rolls_back_earlier_outlet_trial_and_workspace() {
    let mesh = mesh(vec![0.0, 1.0]);
    let mut first = named_species("first", 0.0);
    first.kinetic_sorption = Some(KineticSorption {
        capacity_coefficient: 0.8,
        transfer_rate: 0.4,
    });
    let later = SoluteSpecies {
        name: "later nonlinear".into(),
        decay_rate: 0.0,
        equilibrium_sorption: EquilibriumSorption::Nonlinear {
            fraction: 0.7,
            coefficient: 3.0,
            saturation: 0.2,
            exponent: 0.7,
        },
        kinetic_sorption: None,
    };
    let outlet = SoluteBoundaries {
        lower: Mode::AdvectiveOutflow,
        upper: Mode::TotalFlux { flux: 0.0 },
    };
    let config = SoluteConfig::new_with_species_boundaries(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![first, later],
        vec![outlet; 2],
        1,
        &[],
    )
    .unwrap()
    .with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    let mut state = SoluteState::new(
        &config,
        &mesh,
        0.0,
        vec![1.0, 0.5, 0.7, 0.4],
        vec![0.3, 0.2, 0.0, 0.0],
    )
    .unwrap();
    let mut workspace = SoluteWorkspace::new(2, 2).unwrap();
    let theta = [0.5; 2];
    let faces = [-0.2, 0.0, 0.0];
    let dispersion = [0.0];
    let density = [1.0; 2];
    let successful_input = SoluteTransportInput {
        accepted_water: AcceptedWaterSnapshot {
            accepted_time: 1.0,
            water_contents: &theta,
            face_fluxes: &faces,
        },
        start_water_contents: &theta,
        face_dispersion: &dispersion,
        timestep: 1.0,
        minimum_pivot: 1e-12,
    };
    advance_solute_step_with_processes(
        &mut state,
        &mut workspace,
        &mesh,
        &config,
        successful_input,
        Some(SoluteProcessInput {
            bulk_density: &density,
            max_iterations: 100,
            nonlinear_tolerance: 1e-12,
        }),
    )
    .unwrap();
    assert!(state.kinetic_sorbed().iter().any(|pool| *pool > 0.0));
    assert!(state
        .accounting()
        .cumulative_boundary_mass()
        .iter()
        .any(|mass| *mass != 0.0));
    let original = state.clone();
    let failing_input = SoluteTransportInput {
        accepted_water: AcceptedWaterSnapshot {
            accepted_time: state.time() + 1.0,
            water_contents: &theta,
            face_fluxes: &faces,
        },
        start_water_contents: &theta,
        face_dispersion: &dispersion,
        timestep: 1.0,
        minimum_pivot: 1e-12,
    };
    let result = advance_solute_step_with_processes(
        &mut state,
        &mut workspace,
        &mesh,
        &config,
        failing_input,
        Some(SoluteProcessInput {
            bulk_density: &density,
            max_iterations: 1,
            nonlinear_tolerance: 1e-30,
        }),
    );
    assert!(matches!(
        result,
        Err(SoluteStepError::NonConverged { species: 1, .. })
    ));
    assert_eq!(state, original);
    assert_eq!(workspace.trial_time(), original.time());
    assert_eq!(workspace.trial_concentrations(), original.concentrations());
    assert_eq!(workspace.trial_kinetic_sorbed(), original.kinetic_sorbed());
    assert_eq!(workspace.trial_accounting(), original.accounting());
}

#[test]
fn each_chain_species_exports_its_own_solved_mass_and_mixed_concentration_remainder_closes() {
    let mesh = mesh(vec![0.0, 2.0]);
    let species = vec![
        named_species("NH4", 0.2),
        named_species("NO2", 0.3),
        named_species("NO3", 0.0),
    ];
    let outlet = SoluteBoundaries {
        lower: Mode::AdvectiveOutflow,
        upper: Mode::TotalFlux { flux: 0.0 },
    };
    let parent_inlet = SoluteBoundaries {
        lower: Mode::AdvectiveOutflow,
        upper: Mode::TotalFlux { flux: 0.4 },
    };
    let chain_config = SoluteConfig::new_with_species_boundaries(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        species,
        vec![parent_inlet, outlet, outlet],
        1,
        &[],
    )
    .unwrap()
    .with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    let old = vec![2.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let mut state = SoluteState::new(&chain_config, &mesh, 0.0, old.clone(), vec![0.0; 6]).unwrap();
    advance(
        &mut state,
        &mesh,
        &chain_config,
        &[0.5; 2],
        &[-0.5, 0.0, 0.0],
        &[0.0],
        true,
    )
    .unwrap();
    assert!(state.concentrations()[2] > 0.0);
    assert!(state.concentrations()[4] > 0.0);
    let new = state.concentrations();
    for (species, lower) in [new[0], new[2], new[4]].into_iter().enumerate() {
        let inlet = if species == 0 { 0.4 } else { 0.0 };
        let independently_integrated_storage = 0.5
            * (new[species * 2] + new[species * 2 + 1] - old[species * 2] - old[species * 2 + 1]);
        let independently_computed_exchange = inlet - 0.5 * lower;
        let incoming_reaction = if species == 0 {
            0.0
        } else {
            [0.2, 0.3][species - 1] * 0.5 * (new[(species - 1) * 2] + new[(species - 1) * 2 + 1])
        };
        let outgoing_reaction =
            [0.2, 0.3, 0.0][species] * 0.5 * (new[species * 2] + new[species * 2 + 1]);
        let independently_computed_reaction = incoming_reaction - outgoing_reaction;
        let balance = state.accounting().last_step()[species];
        close(balance.storage_change, independently_integrated_storage);
        close(balance.boundary_exchange, independently_computed_exchange);
        close(balance.reaction_change, independently_computed_reaction);
        let scale = independently_integrated_storage.abs()
            + independently_computed_exchange.abs()
            + independently_computed_reaction.abs();
        let bound = 64.0 * f64::EPSILON * scale.max(1.0);
        assert!(
            (independently_integrated_storage
                - independently_computed_exchange
                - independently_computed_reaction)
                .abs()
                <= bound
        );
    }

    let mixed = config(
        Mode::AdvectiveOutflow,
        Mode::Concentration { value: 0.0 },
        0.0,
        EquilibriumSorption::None,
    );
    for process in [false, true] {
        let mixed_old = [1.0, 0.25];
        let mut mixed_state =
            SoluteState::new(&mixed, &mesh, 0.0, mixed_old.to_vec(), vec![0.0; 2]).unwrap();
        advance(
            &mut mixed_state,
            &mesh,
            &mixed,
            &[0.5; 2],
            &[-0.5, 0.0, 0.0],
            &[0.0],
            process,
        )
        .unwrap();
        let balance = mixed_state.accounting().last_step()[0];
        let independent_storage = 0.5
            * (mixed_state.concentrations()[0] + mixed_state.concentrations()[1]
                - mixed_old[0]
                - mixed_old[1]);
        close(independent_storage, balance.boundary_exchange);
        let explicit_outlet = -0.5 * mixed_state.concentrations()[0];
        assert!(balance.boundary_exchange < explicit_outlet);
    }
}

#[test]
fn shared_outlet_broadcast_matches_explicit_species_pairs() {
    let mesh = mesh(vec![0.0, 1.0]);
    let species = vec![named_species("first", 0.0), named_species("second", 0.0)];
    let outlet = SoluteBoundaries {
        lower: Mode::AdvectiveOutflow,
        upper: Mode::TotalFlux { flux: 0.0 },
    };
    let units = SoluteUnits {
        concentration: "mg/L".into(),
        sorbed_mass: "mg/kg".into(),
    };
    let broadcast = SoluteConfig::new(units.clone(), species.clone(), outlet, 1, &[])
        .unwrap()
        .with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    let explicit =
        SoluteConfig::new_with_species_boundaries(units, species, vec![outlet; 2], 1, &[])
            .unwrap()
            .with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    for process in [false, true] {
        let mut broadcast_state = SoluteState::new(
            &broadcast,
            &mesh,
            0.0,
            vec![1.0, 0.5, 0.25, 0.75],
            vec![0.0; 4],
        )
        .unwrap();
        let mut explicit_state = SoluteState::new(
            &explicit,
            &mesh,
            0.0,
            vec![1.0, 0.5, 0.25, 0.75],
            vec![0.0; 4],
        )
        .unwrap();
        for (state, config) in [
            (&mut broadcast_state, &broadcast),
            (&mut explicit_state, &explicit),
        ] {
            advance(
                state,
                &mesh,
                config,
                &[0.5; 2],
                &[-0.25, 0.0, 0.0],
                &[0.0],
                process,
            )
            .unwrap();
        }
        assert_eq!(
            broadcast_state.concentrations(),
            explicit_state.concentrations()
        );
        assert_eq!(broadcast_state.accounting(), explicit_state.accounting());
    }
}

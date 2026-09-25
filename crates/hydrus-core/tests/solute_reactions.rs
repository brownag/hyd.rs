use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::solute::SoluteProcessInput;
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

fn config(species: Vec<SoluteSpecies>) -> SoluteConfig {
    SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        species,
        SoluteBoundaries {
            lower: Boundary::TotalFlux { flux: 0.0 },
            upper: Boundary::TotalFlux { flux: 0.0 },
        },
        1,
        &[],
    )
    .unwrap()
}

fn make_state(config: &SoluteConfig, concentrations: &[f64], sorbed: &[f64]) -> SoluteState {
    SoluteState::new(
        config,
        &mesh(),
        0.0,
        concentrations.to_vec(),
        sorbed.to_vec(),
    )
    .unwrap()
}

fn run(
    state: &mut SoluteState,
    config: &SoluteConfig,
    water: [f64; 3],
    bulk_density: [f64; 3],
    dispersion: [f64; 2],
    timestep: f64,
    nonlinear_policy: (usize, f64),
) -> Result<(), SoluteStepError> {
    let mesh = mesh();
    let mut workspace = SoluteWorkspace::new(3, config.species().len()).unwrap();
    let accepted_time = state.time() + timestep;
    hydrus_core::solute::advance_solute_step_with_processes(
        state,
        &mut workspace,
        &mesh,
        config,
        SoluteTransportInput {
            accepted_water: AcceptedWaterSnapshot {
                accepted_time,
                water_contents: &water,
                face_fluxes: &[0.0; 4],
            },
            face_dispersion: &dispersion,
            timestep,
            minimum_pivot: 1e-12,
        },
        Some(SoluteProcessInput {
            bulk_density: &bulk_density,
            max_iterations: nonlinear_policy.0,
            nonlinear_tolerance: nonlinear_policy.1,
        }),
    )
}

fn eq_sorbed(isotherm: &EquilibriumSorption, concentration: f64) -> f64 {
    match *isotherm {
        EquilibriumSorption::None => 0.0,
        EquilibriumSorption::Linear {
            distribution_coefficient,
        } => distribution_coefficient * concentration,
        EquilibriumSorption::Nonlinear {
            fraction,
            coefficient,
            saturation,
            exponent,
        } => {
            let power = concentration.powf(exponent);
            fraction * coefficient * power / (1.0 + saturation * power)
        }
    }
}

fn total_mass(
    state: &SoluteState,
    config: &SoluteConfig,
    water: [f64; 3],
    bulk_density: [f64; 3],
) -> f64 {
    let widths = [0.5, 1.0, 0.5];
    let mut total = 0.0;
    for species_index in 0..config.species().len() {
        for node in 0..3 {
            let index = species_index * 3 + node;
            let concentration = state.concentrations()[index];
            let sorbed = state.kinetic_sorbed()[index]
                + eq_sorbed(
                    &config.species()[species_index].equilibrium_sorption,
                    concentration,
                );
            total += widths[node] * (water[node] * concentration + bulk_density[node] * sorbed);
        }
    }
    total
}

fn close(left: f64, right: f64, tolerance: f64) {
    assert!((left - right).abs() <= tolerance, "{left} != {right}");
}

#[test]
fn sequential_decay_transfers_mass_in_species_order_and_accounts_terminal_loss() {
    // C is concentration [mg/L], M is total dissolved+sorbed mass per soil volume,
    // and each k is inverse days. Implicit chain: M_i(new)-M_i(old) =
    // dt * (k_(i-1)*M_(i-1,new) - k_i*M_i(new)).
    let config = config(vec![
        species("parent", 0.5, EquilibriumSorption::None, None),
        species("daughter", 0.25, EquilibriumSorption::None, None),
        species("terminal", 0.0, EquilibriumSorption::None, None),
    ]);
    let water = [0.3; 3];
    let density = [1.0; 3];
    let initial_c = [1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let mut state = make_state(&config, &initial_c, &[0.0; 9]);
    let initial_mass = total_mass(&state, &config, water, density);
    run(
        &mut state,
        &config,
        water,
        density,
        [0.0; 2],
        0.2,
        (20, 1e-10),
    )
    .unwrap();

    assert!(state.concentrations()[3] > 0.0);
    assert!(state.concentrations()[6] > 0.0);
    close(
        total_mass(&state, &config, water, density),
        initial_mass,
        1e-12,
    );
    let reactions = state.accounting().cumulative_reaction_mass();
    assert!(reactions[0] < 0.0);
    assert!(reactions[1] > 0.0);
    assert!(reactions[2] > 0.0);
    close(reactions.iter().sum(), 0.0, 1e-12);
    let widths = [0.5, 1.0, 0.5];
    for (species_index, reaction_mass) in reactions.iter().enumerate() {
        let mass_change: f64 = widths
            .iter()
            .enumerate()
            .map(|(node, width)| {
                let index = species_index * 3 + node;
                width * water[node] * (state.concentrations()[index] - initial_c[index])
            })
            .sum();
        close(*reaction_mass, mass_change, 1e-12);
    }
}

#[test]
fn linear_equilibrium_sorption_retards_transport_and_conserves_total_mass() {
    // S=Kd*C with Kd [mg/kg per mg/L]; rho_b [kg/L] makes rho_b*S
    // compatible with aqueous storage theta*C [mg/L].
    let sorbing = config(vec![species(
        "linear",
        0.0,
        EquilibriumSorption::Linear {
            distribution_coefficient: 2.0,
        },
        None,
    )]);
    let nonsorbing = config(vec![species(
        "linear",
        0.0,
        EquilibriumSorption::None,
        None,
    )]);
    let water = [0.3; 3];
    let density = [1.0; 3];
    let mut retarded = make_state(&sorbing, &[1.0, 0.2, 0.1], &[0.0; 3]);
    let mut free = make_state(&nonsorbing, &[1.0, 0.2, 0.1], &[0.0; 3]);
    let before = total_mass(&retarded, &sorbing, water, density);
    run(
        &mut retarded,
        &sorbing,
        water,
        density,
        [0.2; 2],
        0.1,
        (20, 1e-10),
    )
    .unwrap();
    run(
        &mut free,
        &nonsorbing,
        water,
        density,
        [0.2; 2],
        0.1,
        (20, 1e-10),
    )
    .unwrap();
    close(
        total_mass(&retarded, &sorbing, water, density),
        before,
        1e-12,
    );
    assert!(retarded.concentrations()[1] < free.concentrations()[1]);
}

#[test]
fn nonlinear_equilibrium_sorption_conserves_mass_and_has_expected_retardation() {
    // S=f*K*C^n/(1+nu*C^n): K has sorbed/concentration^n units,
    // nu has concentration^-n units; f and n are dimensionless.
    let sorbing = config(vec![species(
        "nonlinear",
        0.0,
        EquilibriumSorption::Nonlinear {
            fraction: 0.8,
            coefficient: 1.2,
            saturation: 0.25,
            exponent: 1.4,
        },
        None,
    )]);
    let water = [0.3; 3];
    let density = [1.0; 3];
    let mut state = make_state(&sorbing, &[1.0, 0.4, 0.2], &[0.0; 3]);
    let nonsorbing = config(vec![species(
        "nonlinear",
        0.0,
        EquilibriumSorption::None,
        None,
    )]);
    let mut free = make_state(&nonsorbing, &[1.0, 0.4, 0.2], &[0.0; 3]);
    let before = total_mass(&state, &sorbing, water, density);
    run(
        &mut state,
        &sorbing,
        water,
        density,
        [0.15; 2],
        0.1,
        (60, 1e-10),
    )
    .unwrap();
    run(
        &mut free,
        &nonsorbing,
        water,
        density,
        [0.15; 2],
        0.1,
        (60, 1e-10),
    )
    .unwrap();
    close(total_mass(&state, &sorbing, water, density), before, 1e-10);
    assert!(state.concentrations()[1] < free.concentrations()[1]);
}

#[test]
fn kinetic_transfer_moves_toward_equilibrium_and_conserves_dissolved_plus_sorbed_mass() {
    // dS/dt=omega*(Kd*C-S); omega [day^-1], Kd [sorbed/concentration].
    // The backward-Euler exchange is internal, so integrated theta*C+rho_b*S is conserved.
    let config = config(vec![species(
        "kinetic",
        0.0,
        EquilibriumSorption::None,
        Some(KineticSorption {
            capacity_coefficient: 2.0,
            transfer_rate: 1.0,
        }),
    )]);
    let water = [0.3; 3];
    let density = [1.0; 3];
    let mut adsorption = make_state(&config, &[1.0; 3], &[0.0; 3]);
    let before = total_mass(&adsorption, &config, water, density);
    run(
        &mut adsorption,
        &config,
        water,
        density,
        [0.0; 2],
        0.2,
        (20, 1e-10),
    )
    .unwrap();
    assert!(adsorption.kinetic_sorbed()[0] > 0.0);
    assert!(adsorption.concentrations()[0] < 1.0);
    close(
        total_mass(&adsorption, &config, water, density),
        before,
        1e-12,
    );

    let mut desorption = make_state(&config, &[0.1; 3], &[1.0; 3]);
    let before = total_mass(&desorption, &config, water, density);
    run(
        &mut desorption,
        &config,
        water,
        density,
        [0.0; 2],
        0.2,
        (20, 1e-10),
    )
    .unwrap();
    assert!(desorption.kinetic_sorbed()[0] < 1.0);
    assert!(desorption.concentrations()[0] > 0.1);
    close(
        total_mass(&desorption, &config, water, density),
        before,
        1e-12,
    );
}

#[test]
fn zero_sorption_and_zero_transfer_are_valid_limits() {
    let isotherm_limits = [
        EquilibriumSorption::Linear {
            distribution_coefficient: 0.0,
        },
        EquilibriumSorption::Nonlinear {
            fraction: 0.0,
            coefficient: 4.0,
            saturation: 2.0,
            exponent: 1.2,
        },
    ];
    for isotherm in isotherm_limits {
        let config = config(vec![species("zero", 0.0, isotherm, None)]);
        let mut state = make_state(&config, &[0.5; 3], &[0.0; 3]);
        run(
            &mut state,
            &config,
            [0.3; 3],
            [1.0; 3],
            [0.0; 2],
            0.1,
            (20, 1e-10),
        )
        .unwrap();
        assert!(state.concentrations().iter().all(|value| value.is_finite()));
    }
    let config = config(vec![species(
        "no-transfer",
        0.0,
        EquilibriumSorption::None,
        Some(KineticSorption {
            capacity_coefficient: 3.0,
            transfer_rate: 0.0,
        }),
    )]);
    let mut state = make_state(&config, &[0.5; 3], &[0.7; 3]);
    run(
        &mut state,
        &config,
        [0.3; 3],
        [1.0; 3],
        [0.0; 2],
        0.1,
        (20, 1e-10),
    )
    .unwrap();
    assert_eq!(state.kinetic_sorbed(), &[0.7; 3]);
}

#[test]
fn invalid_parameters_deferred_branches_and_failed_trials_are_typed_and_atomic() {
    let units = SoluteUnits {
        concentration: "mg/L".into(),
        sorbed_mass: "mg/kg".into(),
    };
    let boundaries = SoluteBoundaries {
        lower: Boundary::TotalFlux { flux: 0.0 },
        upper: Boundary::TotalFlux { flux: 0.0 },
    };
    assert!(matches!(
        SoluteConfig::new(
            units.clone(),
            vec![species("bad-rate", -1.0, EquilibriumSorption::None, None)],
            boundaries,
            1,
            &[]
        ),
        Err(SoluteConfigError::Invalid {
            name: "decay rate",
            ..
        })
    ));
    assert!(matches!(
        SoluteConfig::new(
            units.clone(),
            vec![species(
                "bad-kinetic",
                0.0,
                EquilibriumSorption::None,
                Some(KineticSorption {
                    capacity_coefficient: 1.0,
                    transfer_rate: f64::NAN
                })
            )],
            boundaries,
            1,
            &[]
        ),
        Err(SoluteConfigError::Invalid {
            name: "kinetic sorption",
            ..
        })
    ));
    assert!(matches!(
        SoluteConfig::new(
            units.clone(),
            vec![species(
                "bad-isotherm",
                0.0,
                EquilibriumSorption::Nonlinear {
                    fraction: 1.0,
                    coefficient: 1.0,
                    saturation: 0.0,
                    exponent: 0.0
                },
                None
            )],
            boundaries,
            1,
            &[]
        ),
        Err(SoluteConfigError::Invalid {
            name: "equilibrium sorption",
            ..
        })
    ));
    for feature in [
        DeferredSoluteFeature::DualPorosityWater,
        DeferredSoluteFeature::Bacteria,
        DeferredSoluteFeature::Filtration,
        DeferredSoluteFeature::DensityDependentTransport,
        DeferredSoluteFeature::SpecialChemistry,
    ] {
        assert!(matches!(
            SoluteConfig::new(
                units.clone(),
                vec![species("rejected", 0.0, EquilibriumSorption::None, None)],
                boundaries,
                1,
                &[feature]
            ),
            Err(SoluteConfigError::Unsupported { .. })
        ));
    }

    let config = config(vec![species(
        "nonlinear-fail",
        0.0,
        EquilibriumSorption::Nonlinear {
            fraction: 1.0,
            coefficient: 2.0,
            saturation: 0.1,
            exponent: 1.7,
        },
        None,
    )]);
    let water = [0.3; 3];
    let mut state = make_state(&config, &[1.0, 0.5, 0.2], &[0.0; 3]);
    let original = state.clone();
    let error = run(
        &mut state,
        &config,
        water,
        [1.0; 3],
        [0.1; 2],
        0.2,
        (1, 1e-14),
    )
    .unwrap_err();
    assert!(matches!(error, SoluteStepError::NonConverged { .. }));
    assert_eq!(state, original);

    let mut state = make_state(&config, &[1.0, 0.5, 0.2], &[0.0; 3]);
    let original = state.clone();
    let error = run(
        &mut state,
        &config,
        water,
        [f64::NAN; 3],
        [0.1; 2],
        0.2,
        (20, 1e-10),
    )
    .unwrap_err();
    assert!(matches!(error, SoluteStepError::Configuration(_)));
    assert_eq!(state, original);
}

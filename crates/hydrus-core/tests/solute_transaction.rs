use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::{
    EquilibriumSorption, SoluteBoundaries, SoluteBoundaryMode, SoluteConfig, SoluteConfigError,
    SoluteSpecies, SoluteState, SoluteUnits, SoluteWorkspace,
};

fn config() -> SoluteConfig {
    SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![
            SoluteSpecies {
                name: "parent".into(),
                decay_rate: 0.1,
                equilibrium_sorption: EquilibriumSorption::None,
                kinetic_sorption: None,
            },
            SoluteSpecies {
                name: "daughter".into(),
                decay_rate: 0.0,
                equilibrium_sorption: EquilibriumSorption::None,
                kinetic_sorption: None,
            },
        ],
        SoluteBoundaries {
            upper: SoluteBoundaryMode::Concentration { value: 0.0 },
            lower: SoluteBoundaryMode::TotalFlux { flux: 0.0 },
        },
        1,
        &[],
    )
    .unwrap()
}

fn mesh() -> Mesh1D {
    Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap()
}

fn state() -> SoluteState {
    SoluteState::new(
        &config(),
        &mesh(),
        2.0,
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
    )
    .unwrap()
}

#[test]
fn rejects_state_and_workspace_dimension_mismatches_before_mutation() {
    let initial = SoluteState::new(&config(), &mesh(), 0.0, vec![0.0; 5], vec![0.0; 5]);
    assert!(matches!(
        initial,
        Err(SoluteConfigError::Invalid {
            name: "state dimensions",
            ..
        })
    ));

    let accepted = state();
    let original = accepted.clone();
    let mut workspace = SoluteWorkspace::new(2, 2).unwrap();
    assert!(matches!(
        workspace.begin_trial(&accepted),
        Err(SoluteConfigError::Invalid {
            name: "workspace dimensions",
            ..
        })
    ));
    assert_eq!(accepted, original);
}

#[test]
fn successful_trial_promotes_all_pools_time_and_accounting() {
    let mut accepted = state();
    let mut workspace = SoluteWorkspace::new(3, 2).unwrap();
    workspace.begin_trial(&accepted).unwrap();
    workspace.trial_concentrations_mut()[0] = 8.0;
    workspace.trial_kinetic_sorbed_mut()[5] = 1.5;
    workspace.trial_boundary_mass_mut()[0] = 2.25;
    workspace.trial_reaction_mass_mut()[1] = -0.75;

    workspace.accept_trial(&mut accepted, 2.5).unwrap();
    assert_eq!(accepted.time(), 2.5);
    assert_eq!(accepted.concentrations()[0], 8.0);
    assert_eq!(accepted.kinetic_sorbed()[5], 1.5);
    assert_eq!(accepted.accounting().accepted_steps(), 1);
    assert_eq!(accepted.accounting().cumulative_boundary_mass()[0], 2.25);
    assert_eq!(accepted.accounting().cumulative_reaction_mass()[1], -0.75);
}

#[test]
fn rejected_trial_restores_every_pool_and_accounting_field_and_reuses_workspace() {
    let mut accepted = state();
    let original = accepted.clone();
    let mut workspace = SoluteWorkspace::new(3, 2).unwrap();
    workspace.begin_trial(&accepted).unwrap();
    workspace.trial_concentrations_mut().fill(99.0);
    workspace.trial_kinetic_sorbed_mut().fill(88.0);
    workspace.set_trial_accepted_steps(77);
    workspace.trial_boundary_mass_mut().fill(66.0);
    workspace.trial_reaction_mass_mut().fill(55.0);

    assert!(workspace.accept_trial(&mut accepted, f64::NAN).is_err());
    assert_eq!(accepted, original);
    workspace.reject_trial();
    assert_eq!(workspace.trial_time(), original.time());
    assert_eq!(workspace.trial_concentrations(), original.concentrations());
    assert_eq!(workspace.trial_kinetic_sorbed(), original.kinetic_sorbed());
    assert_eq!(workspace.trial_accounting(), original.accounting());

    workspace.begin_trial(&accepted).unwrap();
    workspace.trial_concentrations_mut()[1] = 10.0;
    workspace.accept_trial(&mut accepted, 3.0).unwrap();
    assert_eq!(accepted.time(), 3.0);
    assert_eq!(accepted.concentrations()[1], 10.0);
    assert_eq!(accepted.accounting().accepted_steps(), 1);
}

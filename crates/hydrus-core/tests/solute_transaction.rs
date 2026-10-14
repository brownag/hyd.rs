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

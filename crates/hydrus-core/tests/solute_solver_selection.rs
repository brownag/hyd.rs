//! Tests verifying SoluteBackend selection, default preservation, and round-tripping.
use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::{
    advance_solute_step, AcceptedWaterSnapshot, EquilibriumSorption, SoluteBackend,
    SoluteBoundaries, SoluteBoundaryMode, SoluteConfig, SoluteSpecies, SoluteStepError,
    SoluteTransportInput, SoluteUnits, SoluteWorkspace,
};

fn base_config() -> SoluteConfig {
    SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![
            SoluteSpecies {
                name: "tracer".into(),
                decay_rate: 0.0,
                equilibrium_sorption: EquilibriumSorption::None,
                kinetic_sorption: None,
            },
            SoluteSpecies {
                name: "daughter".into(),
                decay_rate: 0.05,
                equilibrium_sorption: EquilibriumSorption::Linear {
                    distribution_coefficient: 0.2,
                },
                kinetic_sorption: None,
            },
        ],
        SoluteBoundaries {
            lower: SoluteBoundaryMode::TotalFlux { flux: 1.0 },
            upper: SoluteBoundaryMode::AdvectiveOutflow,
        },
        1,
        &[],
    )
    .unwrap()
}

#[test]
fn default_backend_is_fortran_compatible() {
    let config = base_config();
    assert_eq!(config.backend(), SoluteBackend::FortranCompatible);
}

#[test]
fn all_contains_both_backends() {
    assert_eq!(
        SoluteBackend::ALL,
        &[
            SoluteBackend::FiniteVolumeBackwardEuler,
            SoluteBackend::FortranCompatible,
        ]
    );
}

#[test]
fn backend_selection_round_trips() {
    let default = base_config();
    assert_eq!(default.backend(), SoluteBackend::FortranCompatible);

    let fvbe = default
        .clone()
        .with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    assert_eq!(fvbe.backend(), SoluteBackend::FiniteVolumeBackwardEuler);

    let fortran = fvbe.with_backend(SoluteBackend::FortranCompatible);
    assert_eq!(fortran.backend(), SoluteBackend::FortranCompatible);
}

#[test]
fn clone_and_equality_preserve_backend() {
    let fvbe = base_config().with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    let cloned = fvbe.clone();
    assert_eq!(cloned.backend(), SoluteBackend::FiniteVolumeBackwardEuler);
    assert_eq!(cloned, fvbe);

    let default = base_config();
    assert_ne!(cloned, default);
}

#[test]
fn default_advance_solute_step_uses_fortran_compatible_backend() {
    // FortranCompatible requires at least 3 mesh nodes.
    // A 2-node mesh is accepted by FV/BE but rejected by FortranCompatible.
    let mesh_2nodes = Mesh1D::new(vec![0.0, 1.0], vec![0; 2], None, 1, 0, 0.0).unwrap();
    let config = SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![SoluteSpecies {
            name: "tracer".into(),
            decay_rate: 0.0,
            equilibrium_sorption: EquilibriumSorption::None,
            kinetic_sorption: None,
        }],
        SoluteBoundaries {
            lower: SoluteBoundaryMode::TotalFlux { flux: 1.0 },
            upper: SoluteBoundaryMode::AdvectiveOutflow,
        },
        1,
        &[],
    )
    .unwrap();
    assert_eq!(config.backend(), SoluteBackend::FortranCompatible);

    let mut state =
        hydrus_core::SoluteState::new(&config, &mesh_2nodes, 0.0, vec![1.0, 0.0], vec![0.0; 2])
            .unwrap();
    let mut workspace = SoluteWorkspace::new(2, 1).unwrap();
    let theta = [0.3; 2];
    let flux = [0.0; 3];
    let dispersion = [0.1; 1];

    let err = advance_solute_step(
        &mut state,
        &mut workspace,
        &mesh_2nodes,
        &config,
        SoluteTransportInput {
            accepted_water: AcceptedWaterSnapshot {
                accepted_time: 1.0,
                water_contents: &theta,
                face_fluxes: &flux,
            },
            start_water_contents: &theta,
            face_dispersion: &dispersion,
            timestep: 1.0,
            minimum_pivot: 1e-12,
        },
    );

    match err {
        Err(SoluteStepError::Configuration(hydrus_core::SoluteConfigError::Invalid {
            name,
            reason,
        })) => {
            assert_eq!(name, "mesh");
            assert_eq!(
                reason,
                "Fortran compatibility solver requires at least 3 nodes"
            );
        }
        other => panic!("expected Fortran-compatible 3-node requirement, got {other:?}"),
    }

    // Proves that when explicitly selecting FV/BE, the 2-node mesh succeeds
    let fvbe_config = config.with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    let ok = advance_solute_step(
        &mut state,
        &mut workspace,
        &mesh_2nodes,
        &fvbe_config,
        SoluteTransportInput {
            accepted_water: AcceptedWaterSnapshot {
                accepted_time: 1.0,
                water_contents: &theta,
                face_fluxes: &flux,
            },
            start_water_contents: &theta,
            face_dispersion: &dispersion,
            timestep: 1.0,
            minimum_pivot: 1e-12,
        },
    );
    assert!(ok.is_ok(), "explicit FV/BE succeeds on 2-node mesh: {ok:?}");
}

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
fn default_backend_is_finite_volume_backward_euler() {
    let config = base_config();
    assert_eq!(config.backend(), SoluteBackend::FiniteVolumeBackwardEuler);
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
    assert_eq!(default.backend(), SoluteBackend::FiniteVolumeBackwardEuler);

    let fortran = default
        .clone()
        .with_backend(SoluteBackend::FortranCompatible);
    assert_eq!(fortran.backend(), SoluteBackend::FortranCompatible);

    let fvbe = fortran.with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    assert_eq!(fvbe.backend(), SoluteBackend::FiniteVolumeBackwardEuler);
}

#[test]
fn clone_and_equality_preserve_backend() {
    let fortran = base_config().with_backend(SoluteBackend::FortranCompatible);
    let cloned = fortran.clone();
    assert_eq!(cloned.backend(), SoluteBackend::FortranCompatible);
    assert_eq!(cloned, fortran);

    let default = base_config();
    assert_ne!(cloned, default);
}

#[test]
fn default_advance_solute_step_uses_fv_be_and_explicit_fortran_compatible_is_selectable() {
    // FV/BE accepts a 2-node mesh; FortranCompatible requires at least 3 mesh nodes.
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
    assert_eq!(config.backend(), SoluteBackend::FiniteVolumeBackwardEuler);

    let mut state =
        hydrus_core::SoluteState::new(&config, &mesh_2nodes, 0.0, vec![1.0, 0.0], vec![0.0; 2])
            .unwrap();
    let mut workspace = SoluteWorkspace::new(2, 1).unwrap();
    let theta = [0.3; 2];
    let flux = [0.0; 3];
    let dispersion = [0.1; 1];

    // Proves that default backend (FV/BE) succeeds on a 2-node mesh
    let ok = advance_solute_step(
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
    assert!(ok.is_ok(), "default FV/BE succeeds on 2-node mesh: {ok:?}");

    // Proves that explicit with_backend(SoluteBackend::FortranCompatible) enforces the 3-node requirement
    let fortran_config = config.with_backend(SoluteBackend::FortranCompatible);
    let err = advance_solute_step(
        &mut state,
        &mut workspace,
        &mesh_2nodes,
        &fortran_config,
        SoluteTransportInput {
            accepted_water: AcceptedWaterSnapshot {
                accepted_time: 2.0,
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
}

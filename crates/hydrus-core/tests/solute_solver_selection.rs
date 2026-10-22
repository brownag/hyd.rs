//! Tests verifying SoluteBackend selection, default selection, and round-tripping.
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
    assert_eq!(fortran, default);
}

#[test]
fn clone_and_equality_preserve_backend() {
    let fvbe = base_config().with_backend(SoluteBackend::FiniteVolumeBackwardEuler);
    let cloned = fvbe.clone();
    assert_eq!(cloned.backend(), SoluteBackend::FiniteVolumeBackwardEuler);
    assert_eq!(cloned, fvbe);

    let default = base_config();
    assert_ne!(cloned, default);
    assert_eq!(
        default,
        base_config().with_backend(SoluteBackend::FortranCompatible)
    );
}

#[test]
fn default_advance_solute_step_uses_fortran_compatible_and_explicit_fv_be_is_selectable() {
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
    assert_eq!(config.backend(), SoluteBackend::FortranCompatible);
    let fvbe_config = config
        .clone()
        .with_backend(SoluteBackend::FiniteVolumeBackwardEuler);

    let mut state =
        hydrus_core::SoluteState::new(&config, &mesh_2nodes, 0.0, vec![1.0, 0.0], vec![0.0; 2])
            .unwrap();
    let mut workspace = SoluteWorkspace::new(2, 1).unwrap();
    let theta = [0.3; 2];
    let flux = [0.0; 3];
    let dispersion = [0.1; 1];

    // Proves that explicit with_backend(SoluteBackend::FiniteVolumeBackwardEuler) succeeds on a 2-node mesh
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

    // Proves that the default backend (FortranCompatible) enforces the 3-node requirement
    let err = advance_solute_step(
        &mut state,
        &mut workspace,
        &mesh_2nodes,
        &config,
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

fn tracer_config() -> SoluteConfig {
    SoluteConfig::new(
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
    .unwrap()
}

fn step_once(config: &SoluteConfig) -> (f64, Vec<f64>) {
    let mesh = Mesh1D::new(vec![0.0, 0.25, 0.5, 0.75, 1.0], vec![0; 5], None, 1, 0, 0.0).unwrap();
    let initial = vec![1.0, 0.8, 0.5, 0.2, 0.0];
    let mut state =
        hydrus_core::SoluteState::new(config, &mesh, 0.0, initial, vec![0.0; 5]).unwrap();
    let mut workspace = SoluteWorkspace::new(5, 1).unwrap();
    let theta = [0.3; 5];
    let flux = [0.1; 6];
    let dispersion = [0.05; 4];
    advance_solute_step(
        &mut state,
        &mut workspace,
        &mesh,
        config,
        SoluteTransportInput {
            accepted_water: AcceptedWaterSnapshot {
                accepted_time: 0.5,
                water_contents: &theta,
                face_fluxes: &flux,
            },
            start_water_contents: &theta,
            face_dispersion: &dispersion,
            timestep: 0.5,
            minimum_pivot: 1e-12,
        },
    )
    .unwrap();
    (state.time(), state.concentrations().to_vec())
}

#[test]
fn default_step_is_bit_identical_to_explicit_fortran_compatible_and_differs_from_fv_be() {
    let (default_time, default_conc) = step_once(&tracer_config());
    let (fortran_time, fortran_conc) =
        step_once(&tracer_config().with_backend(SoluteBackend::FortranCompatible));
    let (_, fvbe_conc) =
        step_once(&tracer_config().with_backend(SoluteBackend::FiniteVolumeBackwardEuler));

    assert_eq!(default_time.to_bits(), fortran_time.to_bits());
    assert_eq!(default_conc.len(), fortran_conc.len());
    for (i, (d, f)) in default_conc.iter().zip(&fortran_conc).enumerate() {
        assert_eq!(
            d.to_bits(),
            f.to_bits(),
            "entry {i}: default {d} vs Fortran {f}"
        );
    }
    assert!(
        default_conc
            .iter()
            .zip(&fvbe_conc)
            .any(|(d, v)| d.to_bits() != v.to_bits()),
        "default must differ from FV/BE: {default_conc:?} vs {fvbe_conc:?}"
    );
}

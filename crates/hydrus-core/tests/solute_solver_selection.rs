//! Tests verifying SoluteBackend selection, default preservation, and round-tripping.
use hydrus_core::{
    EquilibriumSorption, SoluteBackend, SoluteBoundaries, SoluteBoundaryMode, SoluteConfig,
    SoluteSpecies, SoluteUnits,
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

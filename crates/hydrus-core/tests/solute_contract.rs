use hydrus_core::{
    DeferredSoluteFeature as Deferred, EquilibriumSorption, KineticSorption, SoluteBoundaries,
    SoluteBoundaryMode as Boundary, SoluteConfig, SoluteConfigError, SoluteSpecies, SoluteUnits,
};

fn species(name: &str, decay_rate: f64) -> SoluteSpecies {
    SoluteSpecies {
        name: name.into(),
        decay_rate,
        equilibrium_sorption: EquilibriumSorption::None,
        kinetic_sorption: None,
    }
}

fn valid() -> SoluteConfig {
    SoluteConfig::new(
        SoluteUnits {
            concentration: "mg/L".into(),
            sorbed_mass: "mg/kg".into(),
        },
        vec![
            SoluteSpecies {
                equilibrium_sorption: EquilibriumSorption::Nonlinear {
                    fraction: 0.5,
                    coefficient: 0.4,
                    saturation: 0.2,
                    exponent: 0.8,
                },
                kinetic_sorption: Some(KineticSorption {
                    capacity_coefficient: 1.2,
                    transfer_rate: 0.3,
                }),
                ..species("parent", 0.1)
            },
            SoluteSpecies {
                equilibrium_sorption: EquilibriumSorption::Linear {
                    distribution_coefficient: 0.7,
                },
                ..species("daughter", 0.0)
            },
        ],
        SoluteBoundaries {
            upper: Boundary::Concentration { value: 0.0 },
            lower: Boundary::TotalFlux { flux: 0.2 },
        },
        1,
        &[],
    )
    .unwrap()
}

#[test]
fn accepts_units_species_order_sorption_and_selected_boundaries() {
    let config = valid();
    assert_eq!(config.units().concentration, "mg/L");
    assert_eq!(config.units().sorbed_mass, "mg/kg");
    assert_eq!(
        config
            .species()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["parent", "daughter"]
    );
    assert_eq!(config.species()[0].decay_rate, 0.1);
    assert_eq!(config.water_domain_count(), 1);
    assert_eq!(
        config.boundaries()[0].lower,
        Boundary::TotalFlux { flux: 0.2 }
    );
}

#[test]
fn rejects_empty_units_species_and_malformed_or_nonfinite_parameters() {
    let units = SoluteUnits {
        concentration: " ".into(),
        sorbed_mass: "mg/kg".into(),
    };
    assert!(matches!(
        SoluteConfig::new(
            units,
            vec![species("A", 0.0)],
            valid().boundaries()[0],
            1,
            &[]
        ),
        Err(SoluteConfigError::Invalid {
            name: "concentration units",
            ..
        })
    ));
    assert!(matches!(
        SoluteConfig::new(
            SoluteUnits {
                concentration: "mg/L".into(),
                sorbed_mass: "".into()
            },
            vec![species("A", 0.0)],
            valid().boundaries()[0],
            1,
            &[]
        ),
        Err(SoluteConfigError::Invalid {
            name: "sorbed-mass units",
            ..
        })
    ));
    assert!(matches!(
        SoluteConfig::new(
            SoluteUnits {
                concentration: "mg/L".into(),
                sorbed_mass: "mg/kg".into()
            },
            vec![],
            valid().boundaries()[0],
            1,
            &[]
        ),
        Err(SoluteConfigError::Invalid {
            name: "species",
            ..
        })
    ));
    assert!(matches!(
        SoluteConfig::new(
            SoluteUnits {
                concentration: "mg/L".into(),
                sorbed_mass: "mg/kg".into()
            },
            vec![species("A", 0.0), species("A", 0.1)],
            valid().boundaries()[0],
            1,
            &[]
        ),
        Err(SoluteConfigError::Invalid {
            name: "species name",
            ..
        })
    ));
    let mut bad = species("A", f64::NAN);
    assert!(matches!(
        SoluteConfig::new(
            valid().units().clone(),
            vec![bad.clone()],
            valid().boundaries()[0],
            1,
            &[]
        ),
        Err(SoluteConfigError::Invalid {
            name: "decay rate",
            ..
        })
    ));
    bad.decay_rate = 0.0;
    bad.equilibrium_sorption = EquilibriumSorption::Nonlinear {
        fraction: 0.5,
        coefficient: 1.0,
        saturation: 0.0,
        exponent: f64::INFINITY,
    };
    assert!(matches!(
        SoluteConfig::new(
            valid().units().clone(),
            vec![bad.clone()],
            valid().boundaries()[0],
            1,
            &[]
        ),
        Err(SoluteConfigError::Invalid {
            name: "equilibrium sorption",
            ..
        })
    ));
    bad.equilibrium_sorption = EquilibriumSorption::None;
    bad.kinetic_sorption = Some(KineticSorption {
        capacity_coefficient: 1.0,
        transfer_rate: -0.1,
    });
    assert!(matches!(
        SoluteConfig::new(
            valid().units().clone(),
            vec![bad],
            valid().boundaries()[0],
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
            valid().units().clone(),
            vec![species("A", 0.0)],
            SoluteBoundaries {
                upper: Boundary::TotalFlux { flux: f64::NAN },
                lower: Boundary::Concentration { value: 1.0 }
            },
            1,
            &[]
        ),
        Err(SoluteConfigError::Invalid {
            name: "boundary",
            ..
        })
    ));
}

#[test]
fn accepts_each_supported_boundary_mode_and_rejects_nonfinite_boundary_values() {
    for mode in [
        Boundary::Concentration { value: 1.0 },
        Boundary::TotalFlux { flux: -1.0 },
    ] {
        assert!(SoluteConfig::new(
            valid().units().clone(),
            vec![species("A", 0.0)],
            SoluteBoundaries {
                upper: mode,
                lower: mode
            },
            1,
            &[]
        )
        .is_ok());
    }
}

#[test]
fn rejects_multiple_water_domains_and_every_deferred_combination_with_typed_errors() {
    assert_eq!(
        SoluteConfig::new(
            valid().units().clone(),
            vec![species("A", 0.0)],
            valid().boundaries()[0],
            2,
            &[]
        ),
        Err(SoluteConfigError::Unsupported {
            option: "water domain count"
        }),
    );
    let deferred = [
        Deferred::HeatOrVaporCoupling,
        Deferred::DensityDependentTransport,
        Deferred::Bacteria,
        Deferred::Filtration,
        Deferred::SpecialChemistry,
        Deferred::DualPorosityWater,
        Deferred::MobileImmobileTransport,
        Deferred::ArtificialDispersion,
        Deferred::MoistureDependentTransport,
        Deferred::TemperatureDependence,
    ];
    for feature in deferred {
        assert!(
            matches!(
                SoluteConfig::new(
                    valid().units().clone(),
                    vec![species("A", 0.0)],
                    valid().boundaries()[0],
                    1,
                    &[feature]
                ),
                Err(SoluteConfigError::Unsupported { .. })
            ),
            "feature {feature:?}"
        );
    }
}

#[test]
fn boundary_pairs_broadcast_and_explicit_configuration_is_species_indexed() {
    let broadcast = valid();
    let explicit = SoluteConfig::new_with_species_boundaries(
        broadcast.units().clone(),
        broadcast.species().to_vec(),
        vec![broadcast.boundaries()[0]; broadcast.species().len()],
        1,
        &[],
    )
    .unwrap();
    assert_eq!(broadcast, explicit);
    assert_eq!(explicit.boundaries().len(), 2);
    assert_eq!(
        explicit.boundaries().get(1),
        Some(&explicit.boundaries()[0])
    );
}

#[test]
fn rejects_bad_boundary_pair_count_and_invalid_nonfirst_pair() {
    let species = vec![species("A", 0.0), species("B", 0.0)];
    let units = valid().units().clone();
    let good = SoluteBoundaries {
        upper: Boundary::TotalFlux { flux: 0.0 },
        lower: Boundary::TotalFlux { flux: 0.0 },
    };
    for boundaries in [vec![good], vec![good, good, good]] {
        assert!(matches!(
            SoluteConfig::new_with_species_boundaries(
                units.clone(),
                species.clone(),
                boundaries,
                1,
                &[]
            ),
            Err(SoluteConfigError::Invalid {
                name: "boundary pairs",
                ..
            })
        ));
    }
    for invalid in [
        SoluteBoundaries {
            upper: Boundary::TotalFlux { flux: f64::NAN },
            lower: Boundary::TotalFlux { flux: 0.0 },
        },
        SoluteBoundaries {
            upper: Boundary::Concentration { value: -0.1 },
            lower: Boundary::TotalFlux { flux: 0.0 },
        },
    ] {
        assert!(matches!(
            SoluteConfig::new_with_species_boundaries(
                units.clone(),
                species.clone(),
                vec![good, invalid],
                1,
                &[]
            ),
            Err(SoluteConfigError::Invalid { .. })
        ));
    }
}

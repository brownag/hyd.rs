use hydrus_core::{
    HydraulicError, HydraulicModelSelector, HydraulicTables, VanGenuchtenModel,
    VanGenuchtenParameters,
};

fn parameters() -> VanGenuchtenParameters {
    VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0).unwrap()
}

#[test]
fn valid_model_is_constructible_only_after_validation() {
    let model = VanGenuchtenModel::new(parameters()).unwrap();
    assert_eq!(model.parameters().n, 1.6);
}

#[test]
fn invalid_and_non_finite_parameters_are_typed() {
    assert!(matches!(
        VanGenuchtenParameters::new(0.5, 0.4, 0.04, 1.6, 1.0),
        Err(HydraulicError::InvalidParameter { .. })
    ));
    assert!(matches!(
        VanGenuchtenParameters::new(0.05, 0.45, f64::NAN, 1.6, 1.0),
        Err(HydraulicError::NonFiniteParameter { name: "alpha" })
    ));
    assert!(matches!(
        VanGenuchtenParameters::new(0.05, 0.45, 0.04, f64::INFINITY, 1.0),
        Err(HydraulicError::NonFiniteParameter { name: "n" })
    ));
}

#[test]
fn selectors_never_fall_back_to_model_zero() {
    assert_eq!(
        HydraulicModelSelector::from_i32(0).unwrap(),
        HydraulicModelSelector::VanGenuchten
    );
    assert!(matches!(
        HydraulicModelSelector::from_i32(8),
        Err(HydraulicError::UnsupportedSelector { selector: 8 })
    ));
    assert!(matches!(
        HydraulicModelSelector::from_i32(9),
        Err(HydraulicError::UnsupportedSelector { selector: 9 })
    ));
    assert!(matches!(
        HydraulicTables::from_selector(9),
        Err(HydraulicError::UnsupportedSelector { selector: 9 })
    ));
}

#[test]
fn table_and_evaluation_boundaries_are_typed() {
    assert!(matches!(
        HydraulicTables::empty(),
        Err(HydraulicError::InvalidTableConfiguration { .. })
    ));
    let model = VanGenuchtenModel::new(parameters()).unwrap();
    assert!(matches!(
        hydrus_core::HydraulicModel::theta(&model, f64::NAN),
        Err(HydraulicError::InvalidEvaluationDomain { .. })
    ));
}

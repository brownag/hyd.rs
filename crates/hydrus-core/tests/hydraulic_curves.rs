use hydrus_core::{
    HydraulicError, HydraulicModel, HydraulicModelSelector, VanGenuchtenModel,
    VanGenuchtenParameters,
};

fn model() -> VanGenuchtenModel {
    VanGenuchtenModel::new(VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0).unwrap())
        .unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-12 + 1e-10 * expected.abs(),
        "{actual} != {expected}"
    );
}

#[test]
fn hand_calculated_and_saturated_points() {
    let m = model();
    close(m.theta(0.0).unwrap(), 0.45);
    close(m.conductivity(0.0).unwrap(), 1.0);
    close(m.capacity(0.0).unwrap(), 0.0);
    let h = -25.0;
    let se = (1.0 + (0.04 * 25.0_f64).powf(1.6)).powf(-(1.0 - 1.0 / 1.6));
    close(m.theta(h).unwrap(), 0.05 + 0.4 * se);
}

#[test]
fn finite_dry_points_and_inverse_consistency() {
    let m = model();
    for head in [-0.01, -1.0, -100.0, -1.0e6] {
        let evaluation = m.evaluate(head).unwrap();
        assert!(
            evaluation.theta.is_finite()
                && evaluation.conductivity.is_finite()
                && evaluation.capacity.is_finite()
        );
        let recovered = m.head(evaluation.theta).unwrap();
        close(recovered, head);
    }
    assert_eq!(m.head(0.05).unwrap(), -1.0e37);
}

#[test]
fn properties_hold_over_finite_domain() {
    let m = model();
    let mut previous_theta = 0.45;
    let mut previous_k = 1.0;
    for index in 0..1000 {
        let head = -10f64.powf(-3.0 + index as f64 * 6.0 / 999.0);
        let evaluation = m.evaluate(head).unwrap();
        assert!((0.05..=0.45).contains(&evaluation.theta));
        assert!(evaluation.conductivity >= 0.0 && evaluation.capacity >= 0.0);
        assert!(evaluation.theta <= previous_theta + 1e-14);
        assert!(evaluation.conductivity <= previous_k + 1e-14);
        previous_theta = evaluation.theta;
        previous_k = evaluation.conductivity;
    }
}

#[test]
fn invalid_domains_and_selectors_are_typed() {
    let m = model();
    for head in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            m.evaluate(head),
            Err(HydraulicError::InvalidEvaluationDomain { .. })
        ));
    }
    for theta in [f64::NAN, 0.04, 0.46] {
        assert!(matches!(
            m.head(theta),
            Err(HydraulicError::InvalidEvaluationDomain { .. })
        ));
    }
    assert_eq!(
        HydraulicModelSelector::from_i32(0).unwrap(),
        HydraulicModelSelector::VanGenuchten
    );
    assert!(matches!(
        HydraulicModelSelector::from_i32(1),
        Err(HydraulicError::UnsupportedSelector { selector: 1 })
    ));
}

#[test]
fn source_parameter_constraints_remain_enforced() {
    assert!(VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.0, 1.0).is_err());
    assert!(VanGenuchtenParameters::new(0.05, 0.45, 0.0, 1.6, 1.0).is_err());
    assert!(VanGenuchtenParameters::new(0.45, 0.05, 0.04, 1.6, 1.0).is_err());
}

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

fn model_with_connectivity(pore_connectivity: f64) -> VanGenuchtenModel {
    let parameters = VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0)
        .unwrap()
        .with_pore_connectivity(pore_connectivity)
        .unwrap();
    VanGenuchtenModel::new(parameters).unwrap()
}

#[test]
fn conductivity_uses_pore_connectivity_exponent() {
    // Hand derivation at h = -25 cm (alpha = 0.04, n = 1.6, m = 1 - 1/n = 0.375):
    //   x = alpha * 25 = 1, a = 1 + x^n = 2, Se = a^-m = 2^-0.375 = 0.7711054...
    //   Se^(1/m) = 0.5 exactly, so inner = 1 - 0.5 = 0.5 and
    //   bend = 1 - inner^m = 1 - 0.5^0.375 = 1 - Se = 0.2288946...
    //   Mualem K = Ks * Se^l * bend^2 with Ks = 1 and l = 0.5:
    //   K = sqrt(Se) * bend^2 = 0.8781 * 0.052393 = 0.0460074...
    let m = 1.0 - 1.0 / 1.6;
    let se = 2.0_f64.powf(-m);
    let bend = 1.0 - (1.0 - se.powf(1.0 / m)).powf(m);
    let expected = se.powf(0.5) * bend * bend;
    close(
        model_with_connectivity(0.5).conductivity(-25.0).unwrap(),
        expected,
    );
    close(
        model_with_connectivity(0.5).conductivity(-25.0).unwrap(),
        0.046_007_424_463_463_9,
    );
}

#[test]
fn pore_connectivity_changes_unsaturated_conductivity() {
    // At h = -25 cm, Se = 0.7711 < 1, so Se^0 = 1 and Se^0.5 differ:
    //   l = 0 gives K = bend^2 = 0.052393, l = 0.5 gives K = 0.046007.
    let k_zero = model_with_connectivity(0.0).conductivity(-25.0).unwrap();
    let k_half = model_with_connectivity(0.5).conductivity(-25.0).unwrap();
    close(k_zero, 0.052_392_732_093_209_73);
    assert!((k_zero - k_half).abs() > 1e-3, "{k_zero} vs {k_half}");
}

#[test]
fn conductivity_derivative_matches_central_difference() {
    for pore_connectivity in [0.5, 1.5] {
        let m = model_with_connectivity(pore_connectivity);
        for head in [-25.0_f64, -3.0] {
            let eps = 1e-6 * head.abs();
            let analytic = m.derivatives(head).unwrap().dconductivity_dhead;
            let numeric = (m.conductivity(head + eps).unwrap()
                - m.conductivity(head - eps).unwrap())
                / (2.0 * eps);
            assert!(
                (analytic - numeric).abs() <= 1e-6 * (1.0 + numeric.abs()),
                "l={pore_connectivity} h={head}: {analytic} != {numeric}"
            );
        }
    }
}

#[test]
fn pore_connectivity_is_validated() {
    let base = VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0).unwrap();
    assert_eq!(base.pore_connectivity(), 0.5);
    assert!(base.with_pore_connectivity(0.0).is_ok());
    assert!(base.with_pore_connectivity(10.0).is_ok());
    for invalid in [f64::NAN, f64::INFINITY, -1.0, 11.0] {
        assert!(base.with_pore_connectivity(invalid).is_err(), "{invalid}");
    }
}

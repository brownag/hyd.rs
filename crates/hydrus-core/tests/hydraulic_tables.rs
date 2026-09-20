use hydrus_core::{
    HydraulicError, HydraulicModel, HydraulicTables, VanGenuchtenModel, VanGenuchtenParameters,
};

fn model() -> VanGenuchtenModel {
    VanGenuchtenModel::new(VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0).unwrap())
        .unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-12 + 1e-10 * expected.abs());
}

#[test]
fn grid_endpoints_spacing_and_nodes_are_exact() {
    let table = HydraulicTables::new(&model(), -100.0, -0.01, 5).unwrap();
    assert_eq!(table.heads()[0], -100.0);
    assert_eq!(table.heads()[4], -0.01);
    for window in table.heads().windows(2) {
        close(((-window[1]).log10() - (-window[0]).log10()).abs(), 1.0);
    }
    for (&head, value) in table.heads().iter().zip(table.values()) {
        assert_eq!(*value, model().evaluate(head).unwrap());
    }
}

#[test]
fn interpolation_is_finite_and_preserves_applicable_monotonicity() {
    let table = HydraulicTables::new(&model(), -100.0, -0.01, 101).unwrap();
    let mut previous = table.evaluate(table.heads()[0]).unwrap();
    for head in table.heads().iter().copied() {
        let value = table.evaluate(head).unwrap();
        assert!(
            value.theta.is_finite() && value.conductivity.is_finite() && value.capacity.is_finite()
        );
        assert!(value.theta >= previous.theta);
        assert!(value.conductivity >= previous.conductivity);
        assert!(value.capacity >= 0.0);
        previous = value;
    }
    for head in [-10.0, -0.1] {
        let value = table.evaluate(head).unwrap();
        assert!(
            value.theta.is_finite() && value.conductivity.is_finite() && value.capacity.is_finite()
        );
    }
    let middle = table.evaluate(-1.0).unwrap();
    assert!(middle.theta > table.values()[0].theta && middle.theta < table.values()[100].theta);
}

#[test]
fn endpoints_are_accepted_and_out_of_range_is_typed() {
    let table = HydraulicTables::new(&model(), -100.0, -0.01, 5).unwrap();
    assert_eq!(table.evaluate(-100.0).unwrap(), table.values()[0]);
    assert_eq!(table.evaluate(-0.01).unwrap(), table.values()[4]);
    for head in [f64::NAN, f64::INFINITY, -101.0, 0.0] {
        assert!(matches!(
            table.evaluate(head),
            Err(HydraulicError::InvalidEvaluationDomain { .. })
        ));
    }
}

#[test]
fn construction_and_selector_failures_are_typed() {
    let m = model();
    for (min, max, points) in [
        (0.0, -1.0, 2),
        (-1.0, 0.0, 2),
        (-1.0, -2.0, 2),
        (-1.0, -0.1, 1),
    ] {
        assert!(matches!(
            HydraulicTables::new(&m, min, max, points),
            Err(HydraulicError::InvalidTableConfiguration { .. })
        ));
    }
    for selector in [1, 8, 9] {
        assert!(matches!(
            HydraulicTables::from_selector(selector),
            Err(HydraulicError::UnsupportedSelector { selector: actual }) if actual == selector
        ));
    }
}

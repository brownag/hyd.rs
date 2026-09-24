use hydrus_core::{
    HydraulicError, HydraulicModel, HydraulicModelSelector, HydraulicTables, UnitSymbol,
    VanGenuchtenModel, VanGenuchtenParameters,
};

fn model() -> VanGenuchtenModel {
    VanGenuchtenModel::new(VanGenuchtenParameters::new(0.05, 0.45, 0.04, 1.6, 1.0).unwrap())
        .unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-12 + 1e-10 * expected.abs());
}

fn table(points: usize) -> HydraulicTables {
    HydraulicTables::new(
        &model(),
        -100.0,
        -0.01,
        points,
        UnitSymbol::new("cm").unwrap(),
        UnitSymbol::new("cm/day").unwrap(),
        UnitSymbol::new("1/day").unwrap(),
    )
    .unwrap()
}

#[test]
fn grid_endpoints_spacing_and_nodes_are_exact() {
    let table = table(5);
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
    let table = table(101);
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
    let table = table(5);
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
            HydraulicTables::new(
                &m,
                min,
                max,
                points,
                UnitSymbol::new("cm").unwrap(),
                UnitSymbol::new("cm/day").unwrap(),
                UnitSymbol::new("1/day").unwrap(),
            ),
            Err(HydraulicError::InvalidTableConfiguration { .. })
        ));
    }
    for selector in [1, 8, 9] {
        assert!(matches!(
            HydraulicModelSelector::from_i32(selector),
            Err(HydraulicError::UnsupportedSelector { selector: actual }) if actual == selector
        ));
    }
}

#[test]
fn unit_symbols_are_preserved_and_empty_symbols_are_rejected() {
    let table = table(5);
    assert_eq!(table.head_unit().as_str(), "cm");
    assert_eq!(table.conductivity_unit().as_str(), "cm/day");
    assert_eq!(table.capacity_unit().as_str(), "1/day");
    assert!(matches!(
        UnitSymbol::new("  "),
        Err(HydraulicError::InvalidTableConfiguration { .. })
    ));
}

#[test]
fn derivatives_match_segment_slopes_and_use_deterministic_knot_sides() {
    let table = table(5);
    let heads = table.heads();
    for segment in 0..heads.len() - 1 {
        let h0 = heads[segment];
        let h1 = heads[segment + 1];
        let v0 = table.values()[segment];
        let v1 = table.values()[segment + 1];
        let expected = [
            (v1.theta - v0.theta) / (h1 - h0),
            (v1.conductivity - v0.conductivity) / (h1 - h0),
            (v1.capacity - v0.capacity) / (h1 - h0),
        ];
        let interior = (h0 + h1) * 0.5;
        let derivative = table.derivatives(interior).unwrap();
        close(derivative.dtheta_dhead, expected[0]);
        close(derivative.dconductivity_dhead, expected[1]);
        close(derivative.dcapacity_dhead, expected[2]);

        let epsilon = (h1 - h0).abs() * 1e-3;
        let before = table.evaluate(interior - epsilon).unwrap();
        let after = table.evaluate(interior + epsilon).unwrap();
        close((after.theta - before.theta) / (2.0 * epsilon), expected[0]);
        close(
            (after.conductivity - before.conductivity) / (2.0 * epsilon),
            expected[1],
        );
        close(
            (after.capacity - before.capacity) / (2.0 * epsilon),
            expected[2],
        );
    }

    for knot in 1..heads.len() - 1 {
        let right_width = heads[knot + 1] - heads[knot];
        let right = table.values()[knot + 1];
        let left = table.values()[knot];
        let derivative = table.derivatives(heads[knot]).unwrap();
        close(
            derivative.dtheta_dhead,
            (right.theta - left.theta) / right_width,
        );
        close(
            derivative.dconductivity_dhead,
            (right.conductivity - left.conductivity) / right_width,
        );
        close(
            derivative.dcapacity_dhead,
            (right.capacity - left.capacity) / right_width,
        );
    }
    let last = heads.len() - 1;
    let left_width = heads[last] - heads[last - 1];
    let right = table.values()[last];
    let left = table.values()[last - 1];
    let derivative = table.derivatives(heads[last]).unwrap();
    close(
        derivative.dtheta_dhead,
        (right.theta - left.theta) / left_width,
    );
    close(
        derivative.dconductivity_dhead,
        (right.conductivity - left.conductivity) / left_width,
    );
    close(
        derivative.dcapacity_dhead,
        (right.capacity - left.capacity) / left_width,
    );
}

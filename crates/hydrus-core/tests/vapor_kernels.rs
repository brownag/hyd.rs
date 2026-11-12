use hydrus_core::heat::vapor::kernels::{evaluate, x_latent};
use hydrus_core::heat::vapor::{VaporError, VaporPrimitiveInput};

#[allow(clippy::too_many_arguments)]
fn input<'a>(
    heads: &'a [f64],
    temperatures: &'a [f64],
    theta: &'a [f64],
    conductivity: &'a [f64],
    capacity: &'a [f64],
    materials: &'a [usize],
    porosity: &'a [f64],
    x_conv: f64,
    t_conv: f64,
) -> VaporPrimitiveInput<'a> {
    VaporPrimitiveInput {
        heads,
        temperatures,
        liquid_water_contents: theta,
        liquid_conductivities: conductivity,
        liquid_capacities: capacity,
        material_indices: materials,
        saturated_water_contents: porosity,
        x_conv,
        t_conv,
    }
}

#[test]
fn latent_heat_and_material_values_match_independent_points() {
    let hand_calculated = 2_500_699_015.654_4;
    // Allow only a few f64 rounding ULPs; this is not a scientific tolerance.
    assert!(
        (x_latent(0.0).unwrap() - hand_calculated).abs() <= 2.0 * f64::EPSILON * hand_calculated
    );

    let values = evaluate(&input(
        &[-100.0, -200.0, -300.0],
        &[20.0, 20.0, 20.0],
        &[0.2, 0.25, 0.3],
        &[1.0e-5; 3],
        &[0.1; 3],
        &[0; 3],
        &[0.4],
        100.0,
        86400.0,
    ))
    .unwrap();

    // Independent hand calculation for node 0: h=-100/100=-1 m, T=20 C,
    // theta=0.2, theta_s=0.4, xConv=100 and tConv=86400. The contract gives
    // TK=293.15, Diff=7.139918445663763e-7, rho=998.2685184,
    // rho_vs=0.017286519127355054, Hr=0.9999274917189089, eta=11, and
    // drovs=rho_vs(294.15)-rho_vs(293.15). Substitution into the declared
    // ConLT/ConVh/ConVT/ThetaV/capacity equations yields these expectations;
    // none are taken from Rust or Fortran output.
    let expected = [
        (
            values.liquid_thermal_conductivities[0],
            1.4808179162609544e-5,
        ),
        (values.vapor_head_conductivities[0], 1.0375560054392756e-18),
        (
            values.vapor_thermal_conductivities[0],
            9.469534516152574e-13,
        ),
        (values.vapor_contents[0], 3.463049348540306e-6),
        (
            values.effective_hydraulic_capacities[0],
            0.09999829358621166,
        ),
    ];
    for (actual, hand_value) in expected {
        assert!(
            (actual - hand_value).abs() <= 8.0 * f64::EPSILON * hand_value.abs(),
            "actual {actual:.17e}, hand value {hand_value:.17e}"
        );
    }
    // The input liquid conductivity is hydraulic; ConLT is a thermal
    // conductivity diagnostic. Effective capacity is hydraulic, not thermal.
    assert_eq!(values.liquid_conductivities[0], 1.0e-5);
    assert_ne!(
        values.liquid_thermal_conductivities[0],
        values.liquid_conductivities[0]
    );
    assert_ne!(values.effective_hydraulic_capacities[0], 0.1);
}

#[test]
fn saturated_air_has_zero_vapor_conductivity_and_content() {
    let values = evaluate(&input(
        &[-10.0; 3],
        &[10.0; 3],
        &[0.4; 3],
        &[1.0e-5; 3],
        &[0.1; 3],
        &[0; 3],
        &[0.4],
        1.0,
        1.0,
    ))
    .unwrap();
    assert_eq!(values.vapor_head_conductivities, vec![0.0; 3]);
    assert_eq!(values.vapor_thermal_conductivities, vec![0.0; 3]);
    assert_eq!(values.vapor_contents, vec![0.0; 3]);
}

#[test]
fn head_temperature_and_conversion_change_vapor_properties() {
    let evaluate_at = |head, temperature, x_conv, t_conv| {
        evaluate(&input(
            &[head; 3],
            &[temperature; 3],
            &[0.2; 3],
            &[1.0e-5; 3],
            &[0.1; 3],
            &[0; 3],
            &[0.4],
            x_conv,
            t_conv,
        ))
        .unwrap()
    };
    let base = evaluate_at(-100.0, 10.0, 1.0, 1.0);
    let other_head = evaluate_at(-200.0, 10.0, 1.0, 1.0);
    let other_temp = evaluate_at(-100.0, 20.0, 1.0, 1.0);
    let converted = evaluate_at(-100.0, 10.0, 100.0, 86400.0);
    assert_ne!(base.vapor_contents[0], other_head.vapor_contents[0]);
    assert_ne!(
        base.vapor_thermal_conductivities[0],
        other_temp.vapor_thermal_conductivities[0]
    );
    assert_ne!(
        base.vapor_head_conductivities[0],
        converted.vapor_head_conductivities[0]
    );
}

#[test]
fn invalid_primitive_domains_and_lengths_are_rejected() {
    let invalid_temp = input(
        &[-1.0; 3],
        &[-274.0; 3],
        &[0.2; 3],
        &[1.0e-5; 3],
        &[0.1; 3],
        &[0; 3],
        &[0.4],
        1.0,
        1.0,
    );
    assert!(matches!(
        evaluate(&invalid_temp),
        Err(VaporError::InvalidValue { .. })
    ));

    let bad_material = input(
        &[-1.0; 3],
        &[10.0; 3],
        &[0.2; 3],
        &[1.0e-5; 3],
        &[0.1; 3],
        &[0, 0, 1],
        &[0.4],
        1.0,
        1.0,
    );
    assert!(matches!(
        evaluate(&bad_material),
        Err(VaporError::MaterialIndex { .. })
    ));

    assert!(matches!(
        x_latent(f64::NAN),
        Err(VaporError::NonFinite { .. })
    ));
    assert!(matches!(
        x_latent(-274.0),
        Err(VaporError::InvalidValue { .. })
    ));

    let nonfinite = input(
        &[f64::NAN; 3],
        &[10.0; 3],
        &[0.2; 3],
        &[1.0e-5; 3],
        &[0.1; 3],
        &[0; 3],
        &[0.4],
        1.0,
        1.0,
    );
    assert!(matches!(
        evaluate(&nonfinite),
        Err(VaporError::NonFinite { .. })
    ));

    let wrong_length = input(
        &[-1.0; 3],
        &[10.0; 2],
        &[0.2; 3],
        &[1.0e-5; 3],
        &[0.1; 3],
        &[0; 3],
        &[0.4],
        1.0,
        1.0,
    );
    assert!(matches!(
        evaluate(&wrong_length),
        Err(VaporError::LengthMismatch { .. })
    ));
}

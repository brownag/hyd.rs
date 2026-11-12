use hydrus_core::heat::capacity::TempMaterial;
use hydrus_core::heat::config::{HeatConfig, HeatConfigInput};
use hydrus_core::heat::transport::{
    solve_heat_transport, solve_vapor_heat_transport, HeatTransportError, HeatTransportInput,
};
use hydrus_core::heat::vapor::adjust::{adjust_heat_rows, VaporHeatAdjustmentInput};
use hydrus_core::heat::vapor::{VaporConfig, VaporConfigInput, VaporError};

fn heat_input(vapor: bool) -> HeatConfigInput {
    HeatConfigInput {
        l_vapor: vapor,
        l_en_bal: false,
        i_dual_por: 0,
        l_w_dep: false,
        i_model: 0,
        domain_count: 1,
        k_top_t: 1,
        t_period: 0.0,
        k_bot_t: 1,
        campbell: false,
    }
}

fn vapor_config() -> VaporConfig {
    VaporConfig::new(VaporConfigInput {
        heat: heat_input(true),
        water_enabled: true,
        heat_enabled: true,
        i_enhanc: 1,
        hysteresis: 0,
        chemistry: false,
        density_coupling: false,
        centrifugal: false,
        top_temperature: 30.0,
        bottom_temperature: 10.0,
        x_conv: 1.0,
        t_conv: 1.0,
    })
    .unwrap()
}

fn material() -> TempMaterial {
    TempMaterial::from_tpar([0.5, 0.0, 0.0, 0.5, 0.1, 0.2, 2.0, 0.0, 4.0])
}

fn adjustment<'a>(
    x: &'a [f64],
    temp_old: &'a [f64],
    temp_new: &'a [f64],
    v_old: &'a [f64],
    v_new: &'a [f64],
    theta_v_old: &'a [f64],
    theta_v_new: &'a [f64],
) -> VaporHeatAdjustmentInput<'a> {
    VaporHeatAdjustmentInput {
        coordinates: x,
        temperature_old: temp_old,
        temperature_new_estimate: temp_new,
        vapor_velocity_old: v_old,
        vapor_velocity_new: v_new,
        vapor_content_old: theta_v_old,
        vapor_content_new: theta_v_new,
        timestep: 2.0,
        x_conv: 1.0,
        t_conv: 1.0,
    }
}

#[test]
fn both_tempadj_levels_match_hand_calculated_interior_terms() {
    let x = [0.0, 2.0, 5.0];
    let temp = [10.0, 20.0, 30.0];
    let zero = [0.0; 3];
    let out = adjust_heat_rows(&adjustment(
        &x, &temp, &temp, &[1.0; 3], &[2.0; 3], &zero, &zero,
    ))
    .unwrap();
    // Cv=1.8e6. Source products give F1=10*(3Cv/12)-30*(3Cv/12).
    assert_eq!(out.rhs_level_one[1], -9.0e6);
    assert_eq!(out.lower_level_two[1], -9.0e5);
    assert_eq!(out.diagonal_level_two[1], 0.0);
    assert_eq!(out.upper_level_two[1], 9.0e5);
    assert_eq!(out.rhs_level_two[1], 0.0);
    assert_eq!(out.lower_level_two[0], 0.0);
    assert_eq!(out.upper_level_two[2], 0.0);
}

#[test]
fn nonuniform_central_vapor_gradient_keeps_source_factor_two() {
    let x = [0.0, 2.0, 5.0];
    let temp_old = [10.0, 20.0, 30.0];
    let temp_new = temp_old;
    let zero = [0.0; 3];
    let out = adjust_heat_rows(&adjustment(
        &x,
        &temp_old,
        &temp_new,
        &zero,
        &[0.0, 1.0, 4.0],
        &zero,
        &zero,
    ))
    .unwrap();
    // Central source gradient is 2*(4-0)/(5-0)=1.6; endpoint gradients
    // are 0.5 and 1.0. Substitution into g0 and the nonuniform dxA/dxB
    // RHS weighting gives this independently evaluated hand value.
    assert!((out.rhs_level_two[1] - -4_077_071_847.118_152_6).abs() < 1.0e-5);
}

#[test]
fn zero_vapor_reduces_exactly_and_fixed_temperature_endpoints_remain() {
    let config = vapor_config();
    let heat_only = HeatConfig::new(heat_input(false)).unwrap();
    let x = [0.0, 1.0, 3.0, 6.0, 10.0];
    let indices = [0usize; 5];
    let materials = [material()];
    let temp_old = [20.0; 5];
    let water_old = [0.2; 5];
    let water_new = [0.2; 5];
    let velocity = [0.0; 5];
    let sink = [0.0; 5];
    let base_input = HeatTransportInput {
        x: &x,
        dt: 2.0,
        material_indices: &indices,
        materials: &materials,
        temp_old: &temp_old,
        water_old: &water_old,
        water_new: &water_new,
        velocity_old: &velocity,
        velocity_new: &velocity,
        sink: &sink,
        top_temperature: 30.0,
        bottom_temperature: 10.0,
        x_conv: 1.0,
        t_conv: 1.0,
        minimum_pivot: 1.0e-12,
    };
    let mut expected = [0.0; 5];
    solve_heat_transport(heat_only, base_input, &mut expected).unwrap();

    let zero = [0.0; 5];
    let vapor_input = adjustment(&x, &temp_old, &[20.0; 5], &zero, &zero, &zero, &zero);
    let mut actual = [0.0; 5];
    solve_vapor_heat_transport(config, base_input, &vapor_input, &mut actual).unwrap();
    for (a, e) in actual.iter().zip(expected) {
        assert_eq!(a.to_bits(), e.to_bits());
    }
    assert_eq!(actual[0], 10.0);
    assert_eq!(actual[4], 30.0);
}

#[test]
fn invalid_adjustment_and_pivot_failure_leave_destination_unchanged() {
    let x = [0.0, 1.0, 2.0];
    let temp = [20.0; 3];
    let zero = [0.0; 3];
    let invalid = adjustment(&x, &temp, &temp, &zero, &zero, &zero, &[f64::NAN; 3]);
    assert!(matches!(
        adjust_heat_rows(&invalid),
        Err(VaporError::NonFinite { .. })
    ));

    let config = vapor_config();
    let indices = [0usize; 3];
    let materials = [material()];
    let water = [0.2; 3];
    let sink = [0.0; 3];
    let input = HeatTransportInput {
        x: &x,
        dt: 2.0,
        material_indices: &indices,
        materials: &materials,
        temp_old: &temp,
        water_old: &water,
        water_new: &water,
        velocity_old: &zero,
        velocity_new: &zero,
        sink: &sink,
        top_temperature: 30.0,
        bottom_temperature: 10.0,
        x_conv: 1.0,
        t_conv: 1.0,
        minimum_pivot: f64::MAX,
    };
    let histories = adjustment(&x, &temp, &temp, &zero, &zero, &zero, &zero);
    let mut destination = [77.0; 3];
    assert!(matches!(
        solve_vapor_heat_transport(config, input, &histories, &mut destination),
        Err(HeatTransportError::Tridiagonal(_))
    ));
    assert_eq!(destination, [77.0; 3]);
}

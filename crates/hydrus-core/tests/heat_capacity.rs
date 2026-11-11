use hydrus_core::heat::capacity::{temp_cap, TempCapError, TempCapInput, TempMaterial};

fn assert_close(left: f64, right: f64, tolerance: f64) {
    assert!(
        (left - right).abs() < tolerance,
        "{left} != {right} (tolerance {tolerance})"
    );
}

// Material A, TPar rows 1..9 = [s, o_f, disp, q, other, clay, c_s, c_o, c_w]
//   = [0.5, 0.0, 0.5, 0.5, 0.1, 0.2, 2.0, 0.0, 4.0]
fn material_a() -> TempMaterial {
    TempMaterial::from_tpar([0.5, 0.0, 0.5, 0.5, 0.1, 0.2, 2.0, 0.0, 4.0])
}

// Material B, TPar rows 1..9 = [0.3, 0.1, 0.0, 0.2, 0.3, 0.1, 2.5, 1.0, 4.0]
fn material_b() -> TempMaterial {
    TempMaterial::from_tpar([0.3, 0.1, 0.0, 0.2, 0.3, 0.1, 2.5, 1.0, 4.0])
}

fn default_input<'a>(
    indices: &'a [usize],
    theta: &'a [f64],
    velocity: &'a [f64],
    materials: &'a [TempMaterial],
) -> TempCapInput<'a> {
    TempCapInput {
        material_indices: indices,
        water_content: theta,
        velocity,
        materials,
        campbell: false,
        x_conv: 1.0,
        t_conv: 1.0,
    }
}

// Default branch. Hand formulas:
//   Cap  = c_s*s + c_o*o_f + c_w*th
//   Cond = q + other*th + clay*sqrt(th) + c_w*disp*|v|
#[test]
fn default_branch_matches_hand_values_for_two_materials() {
    let materials = [material_a(), material_b()];
    let indices = [0, 1, 0];
    let theta = [0.25, 0.36, 0.04];
    let velocity = [0.0, 0.0, 0.5];
    let mut cap = [0.0; 3];
    let mut cond = [0.0; 3];

    temp_cap(
        default_input(&indices, &theta, &velocity, &materials),
        &mut cap,
        &mut cond,
    )
    .unwrap();

    // Node 0, A, th=0.25: Cap = 2.0*0.5 + 4.0*0.25 = 2.0.
    //   Cond = 0.5 + 0.1*0.25 + 0.2*sqrt(0.25) = 0.5 + 0.025 + 0.1 = 0.625.
    assert_close(cap[0], 2.0, 1e-12);
    assert_close(cond[0], 0.625, 1e-12);
    // Node 1, B, th=0.36: Cap = 2.5*0.3 + 1.0*0.1 + 4.0*0.36 = 0.75 + 0.1 + 1.44 = 2.29.
    //   Cond = 0.2 + 0.3*0.36 + 0.1*sqrt(0.36) = 0.2 + 0.108 + 0.06 = 0.368.
    assert_close(cap[1], 2.29, 1e-12);
    assert_close(cond[1], 0.368, 1e-12);
    // Node 2, A, th=0.04, |v|=0.5: Cap = 1.0 + 4.0*0.04 = 1.16.
    //   Cond = 0.5 + 0.1*0.04 + 0.2*sqrt(0.04) + 4.0*0.5*0.5 = 0.5 + 0.004 + 0.04 + 1.0 = 1.544.
    assert_close(cap[2], 1.16, 1e-12);
    assert_close(cond[2], 1.544, 1e-12);
}

// Campbell branch. At th=0 the exp term is exactly 1, so XLamb = DD = 0.03 + 0.7*s^2.
#[test]
fn campbell_branch_at_zero_water_content_uses_dd() {
    let materials = [material_a(), material_b()];

    // Node 0, A, th=0, |v|=0.5, x_conv=t_conv=2:
    //   DD = 0.03 + 0.7*0.25 = 0.205. Cond = 0.205*2/2/2/2 = 0.05125, plus 4.0*0.5*0.5 = 1.0.
    //   Cap = 2.0*0.5 + 0 = 1.0.
    let indices = [0];
    let theta = [0.0];
    let velocity = [0.5];
    let mut cap = [0.0];
    let mut cond = [0.0];
    temp_cap(
        TempCapInput {
            material_indices: &indices,
            water_content: &theta,
            velocity: &velocity,
            materials: &materials,
            campbell: true,
            x_conv: 2.0,
            t_conv: 2.0,
        },
        &mut cap,
        &mut cond,
    )
    .unwrap();
    assert_close(cap[0], 1.0, 1e-12);
    assert_close(cond[0], 1.05125, 1e-12);

    // Node 0, B, th=0, |v|=0 (disp=0), x_conv=t_conv=1:
    //   DD = 0.03 + 0.7*0.09 = 0.093. Cond = 0.093. Cap = 2.5*0.3 + 1.0*0.1 = 0.85.
    let indices = [1];
    let velocity = [0.0];
    temp_cap(
        TempCapInput {
            material_indices: &indices,
            water_content: &theta,
            velocity: &velocity,
            materials: &materials,
            campbell: true,
            x_conv: 1.0,
            t_conv: 1.0,
        },
        &mut cap,
        &mut cond,
    )
    .unwrap();
    assert_close(cap[0], 0.85, 1e-12);
    assert_close(cond[0], 0.093, 1e-12);
}

// Campbell branch with CC*th large, so exp(-(CC*th)^4) is below 1e-12.
// Material B, th=0.36, clay=0.1, |v|=0, x_conv=t_conv=1. Hand formulas:
//   AA = (0.57 + 1.73*0.2 + 0.93*0.3)/(1 - 0.74*0.2 - 0.49*0.3) - 2.8*0.3*0.7
//      = 1.195/0.705 - 0.588 = 1.69503546 - 0.588 = 1.10703546
//   BB = 2.8*0.3 = 0.84
//   CC = 1 + 2.6/sqrt(0.1) = 9.22192, so (CC*0.36)^4 = (3.31989)^4 = 121.5 and exp(-121.5) is about 4e-53.
//   XLamb = AA + BB*0.36 = 1.10703546 + 0.3024 = 1.40943546.
//   Cond = 1.40943546. Cap = 2.29.
#[test]
fn campbell_branch_with_negligible_exp_term() {
    let materials = [material_b()];
    let indices = [0];
    let theta = [0.36];
    let velocity = [0.0];
    let mut cap = [0.0];
    let mut cond = [0.0];
    temp_cap(
        TempCapInput {
            material_indices: &indices,
            water_content: &theta,
            velocity: &velocity,
            materials: &materials,
            campbell: true,
            x_conv: 1.0,
            t_conv: 1.0,
        },
        &mut cap,
        &mut cond,
    )
    .unwrap();
    assert_close(cap[0], 2.29, 1e-12);
    // Tolerance 1e-9 covers the hand arithmetic (about 10 significant digits).
    assert_close(cond[0], 1.40943546, 1e-9);
}

#[test]
fn rejects_mismatched_slice_lengths() {
    let materials = [material_a()];
    let indices = [0, 0];
    let theta = [0.25, 0.25];
    let velocity = [0.0, 0.0];
    let mut cap = [0.0, 0.0];
    let mut cond = [0.0, 0.0];

    let short_theta = [0.25];
    let error = temp_cap(
        default_input(&indices, &short_theta, &velocity, &materials),
        &mut cap,
        &mut cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::LengthMismatch {
            name: "water_content",
            expected: 2,
            actual: 1,
        }
    );

    let short_velocity = [0.0];
    let error = temp_cap(
        default_input(&indices, &theta, &short_velocity, &materials),
        &mut cap,
        &mut cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::LengthMismatch {
            name: "velocity",
            expected: 2,
            actual: 1,
        }
    );

    let mut short_cap = [0.0];
    let error = temp_cap(
        default_input(&indices, &theta, &velocity, &materials),
        &mut short_cap,
        &mut cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::LengthMismatch {
            name: "cap",
            expected: 2,
            actual: 1,
        }
    );

    let mut short_cond = [0.0];
    let error = temp_cap(
        default_input(&indices, &theta, &velocity, &materials),
        &mut cap,
        &mut short_cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::LengthMismatch {
            name: "cond",
            expected: 2,
            actual: 1,
        }
    );
}

#[test]
fn rejects_non_finite_inputs_with_index() {
    let materials = [material_a()];
    let indices = [0, 0];
    let velocity = [0.0, 0.0];
    let mut cap = [0.0, 0.0];
    let mut cond = [0.0, 0.0];

    let theta_nan = [0.25, f64::NAN];
    let error = temp_cap(
        default_input(&indices, &theta_nan, &velocity, &materials),
        &mut cap,
        &mut cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::NonFiniteInput {
            name: "water_content",
            index: 1,
        }
    );

    let theta = [0.25, 0.25];
    let velocity_inf = [f64::INFINITY, 0.0];
    let error = temp_cap(
        default_input(&indices, &theta, &velocity_inf, &materials),
        &mut cap,
        &mut cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::NonFiniteInput {
            name: "velocity",
            index: 0,
        }
    );
}

#[test]
fn rejects_non_finite_material_parameter() {
    let mut bad = material_a();
    bad.quartz = f64::NAN;
    let materials = [material_a(), bad];
    let indices = [0];
    let theta = [0.25];
    let velocity = [0.0];
    let mut cap = [0.0];
    let mut cond = [0.0];
    let error = temp_cap(
        default_input(&indices, &theta, &velocity, &materials),
        &mut cap,
        &mut cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::NonFiniteParameter {
            material: 1,
            name: "quartz",
        }
    );
}

#[test]
fn rejects_material_index_out_of_range() {
    let materials = [material_a(), material_b()];
    let indices = [0, 2];
    let theta = [0.25, 0.25];
    let velocity = [0.0, 0.0];
    let mut cap = [0.0, 0.0];
    let mut cond = [0.0, 0.0];
    let error = temp_cap(
        default_input(&indices, &theta, &velocity, &materials),
        &mut cap,
        &mut cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::MaterialIndexOutOfRange {
            index: 1,
            material: 2,
            count: 2,
        }
    );
}

#[test]
fn rejects_zero_capacity() {
    // All capacity terms are zero, so Cap = 0 at any water content.
    let zero = TempMaterial::from_tpar([0.0, 0.0, 0.0, 0.5, 0.1, 0.2, 0.0, 0.0, 0.0]);
    let materials = [zero];
    let indices = [0];
    let theta = [0.25];
    let velocity = [0.0];
    let mut cap = [0.0];
    let mut cond = [0.0];
    let error = temp_cap(
        default_input(&indices, &theta, &velocity, &materials),
        &mut cap,
        &mut cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::NonPositiveCapacity {
            index: 0,
            value: 0.0,
        }
    );
}

#[test]
fn rejects_negative_water_content() {
    let materials = [material_a()];
    let indices = [0];
    let theta = [-0.04];
    let velocity = [0.0];
    let mut cap = [0.0];
    let mut cond = [0.0];
    let error = temp_cap(
        default_input(&indices, &theta, &velocity, &materials),
        &mut cap,
        &mut cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::NegativeWaterContent {
            index: 0,
            value: -0.04,
        }
    );
}

#[test]
fn campbell_rejects_non_positive_conversion_factor() {
    let materials = [material_a()];
    let indices = [0];
    let theta = [0.25];
    let velocity = [0.0];
    let mut cap = [0.0];
    let mut cond = [0.0];
    let error = temp_cap(
        TempCapInput {
            material_indices: &indices,
            water_content: &theta,
            velocity: &velocity,
            materials: &materials,
            campbell: true,
            x_conv: 1.0,
            t_conv: 0.0,
        },
        &mut cap,
        &mut cond,
    )
    .unwrap_err();
    assert_eq!(
        error,
        TempCapError::InvalidConversion {
            name: "t_conv",
            value: 0.0,
        }
    );
}

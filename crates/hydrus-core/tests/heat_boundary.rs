use hydrus_core::heat::boundary::{
    set_up_heat_boundary, BoundaryCoefficients, HeatBoundaryError, HeatBoundaryInput, HeatLevel,
};

// Fixture: N = 3 nodes, x = [0, 1, 2], so every spacing dx = 1.
// cap = cond = 1 everywhere, cw = 1, sink = 0, v = 0, dt = 1.
// temp_old = [10, 20, 30], tBot = 5, tTopA = 40, HeatFl = 2, tPeriod = 0.
// Expected values are worked by hand from TEMPER.FOR:87-175.
struct Mesh {
    x: Vec<f64>,
    temp_old: Vec<f64>,
    v_old: Vec<f64>,
    v_new: Vec<f64>,
    cap: Vec<f64>,
    cond: Vec<f64>,
    sink: Vec<f64>,
}

fn mesh() -> Mesh {
    Mesh {
        x: vec![0.0, 1.0, 2.0],
        temp_old: vec![10.0, 20.0, 30.0],
        v_old: vec![0.0; 3],
        v_new: vec![0.0; 3],
        cap: vec![1.0; 3],
        cond: vec![1.0; 3],
        sink: vec![0.0; 3],
    }
}

fn input(m: &Mesh) -> HeatBoundaryInput<'_> {
    HeatBoundaryInput {
        k_top_t: 1,
        k_bot_t: 1,
        t_top_a: 40.0,
        t_bottom: 5.0,
        t_period: 0.0,
        heat_flux: 2.0,
        dt: 1.0,
        cw: 1.0,
        x: &m.x,
        temp_old: &m.temp_old,
        v_old: &m.v_old,
        v_new: &m.v_new,
        cap: &m.cap,
        cond: &m.cond,
        sink: &m.sink,
    }
}

fn some(d: Option<f64>, expected: f64) {
    let value = d.expect("entry should be assigned");
    assert!((value - expected).abs() < 1e-12, "{value} != {expected}");
}

fn assert_coeffs(
    actual: BoundaryCoefficients,
    b: Option<f64>,
    d: Option<f64>,
    e: Option<f64>,
    f: Option<f64>,
) {
    assert_eq!(actual.b.is_some(), b.is_some(), "b assignment");
    assert_eq!(actual.d.is_some(), d.is_some(), "d assignment");
    assert_eq!(actual.e.is_some(), e.is_some(), "e assignment");
    assert_eq!(actual.f.is_some(), f.is_some(), "f assignment");
    if let Some(v) = b {
        some(actual.b, v);
    }
    if let Some(v) = d {
        some(actual.d, v);
    }
    if let Some(v) = e {
        some(actual.e, v);
    }
    if let Some(v) = f {
        some(actual.f, v);
    }
}

#[test]
fn bottom_dirichlet_sets_d_e_f_at_both_levels() {
    // SetUpHeat :87-90: D(1)=1, E(1)=0, F(1)=tBot. kTopT=1 gives :144-147: B(N)=0, D(N)=1, F(N)=tTopA=40.
    let m = mesh();
    for level in [HeatLevel::First, HeatLevel::Second] {
        let rows = set_up_heat_boundary(
            level,
            HeatBoundaryInput {
                k_bot_t: 1,
                ..input(&m)
            },
        )
        .unwrap();
        assert_coeffs(rows.bottom, None, Some(1.0), Some(0.0), Some(5.0));
        assert_coeffs(rows.top, Some(0.0), Some(1.0), None, Some(40.0));
    }
}

#[test]
fn bottom_sign_class_not_integer_value_selects_dirichlet() {
    // kBotT=3 and kTopT=7 are positive: same rows as code 1 (sign class only).
    let m = mesh();
    let rows = set_up_heat_boundary(
        HeatLevel::Second,
        HeatBoundaryInput {
            k_bot_t: 3,
            k_top_t: 7,
            ..input(&m)
        },
    )
    .unwrap();
    assert_coeffs(rows.bottom, None, Some(1.0), Some(0.0), Some(5.0));
    assert_coeffs(rows.top, Some(0.0), Some(1.0), None, Some(40.0));
}

#[test]
fn bottom_zero_gradient_sets_minus_one_one_zero() {
    // SetUpHeat :108-111: D(1)=-1, E(1)=1, F(1)=0, so T(1)=T(2).
    let m = mesh();
    for level in [HeatLevel::First, HeatLevel::Second] {
        let rows = set_up_heat_boundary(
            level,
            HeatBoundaryInput {
                k_bot_t: 0,
                ..input(&m)
            },
        )
        .unwrap();
        assert_coeffs(rows.bottom, None, Some(-1.0), Some(1.0), Some(0.0));
    }
}

#[test]
fn bottom_flux_level_second_writes_d_and_e_only() {
    // SetUpHeat :92-97, Level 2:
    // D(1) = dx/2/dt*Cap1 + (Cond1+Cond2)/dx/4 = 0.5 + 0.5 = 1.0
    // E(1) = -(Cond1+Cond2)/4/dx = -0.5
    let m = mesh();
    let rows = set_up_heat_boundary(
        HeatLevel::Second,
        HeatBoundaryInput {
            k_bot_t: -1,
            ..input(&m)
        },
    )
    .unwrap();
    assert_coeffs(rows.bottom, None, Some(1.0), Some(-0.5), None);
}

#[test]
fn bottom_flux_level_first_writes_f_only() {
    // SetUpHeat :98-106, Level 1, with vOld = vNew = 0 and Sink = 0:
    // F(1) = TempO1*(0.5 - 0.5) + TempO2*(2/4/1) + tBot*0 = 0 + 20*0.5 = 10.0
    let m = mesh();
    let rows = set_up_heat_boundary(
        HeatLevel::First,
        HeatBoundaryInput {
            k_bot_t: -1,
            ..input(&m)
        },
    )
    .unwrap();
    assert_coeffs(rows.bottom, None, None, None, Some(10.0));
}

#[test]
fn top_dirichlet_is_level_independent() {
    // kTopT=1 (and 7 in the sign-class test): B(N)=0, D(N)=1, F(N)=tTopA=40 at both levels.
    let m = mesh();
    for level in [HeatLevel::First, HeatLevel::Second] {
        let rows = set_up_heat_boundary(
            level,
            HeatBoundaryInput {
                k_top_t: 1,
                ..input(&m)
            },
        )
        .unwrap();
        assert_coeffs(rows.top, Some(0.0), Some(1.0), None, Some(40.0));
    }
}

#[test]
fn top_flux_level_first_heat_flux_positive() {
    // SetUpHeat :163-172 with Cw=1, v=0, Sink=0, dx=x3-x2=1:
    // F(N) = TempO2*((Cond3+Cond2)/4/dx) + TempO3*(dx/2/dt*Cap3 - (Cond2+Cond3)/4/dx)
    //      = 20*0.5 + 30*(0.5 - 0.5) = 10.0
    // then F(N) -= HeatFl (2)          -> 8.0
    // then F(N) -= min(0, HeatFl) (0)  -> 8.0
    let m = mesh();
    let rows = set_up_heat_boundary(
        HeatLevel::First,
        HeatBoundaryInput {
            k_top_t: -1,
            ..input(&m)
        },
    )
    .unwrap();
    assert_coeffs(rows.top, None, None, None, Some(8.0));
}

#[test]
fn top_flux_level_first_heat_flux_negative() {
    // Same F(N) = 10.0 as above. HeatFl = -3:
    // F(N) -= -3 -> 13.0; F(N) -= min(0,-3) = -3 -> 16.0
    let m = mesh();
    let rows = set_up_heat_boundary(
        HeatLevel::First,
        HeatBoundaryInput {
            k_top_t: -1,
            heat_flux: -3.0,
            ..input(&m)
        },
    )
    .unwrap();
    assert_coeffs(rows.top, None, None, None, Some(16.0));
}

#[test]
fn top_flux_level_second_with_advection() {
    // SetUpHeat :154-161 with Cw=2, vNew = [0, 0, 6], Sink = 0, dx = 1:
    // B(N) = -(Cond2+Cond3)/4/dx - Cw*(vNew3 + 2*vNew2)/12 = -0.5 - 2*6/12 = -1.5
    // D(N) = dx/2/dt*Cap3 + (Cond2+Cond3)/4/dx - Cw*(2*vNew3 + vNew2)/12
    //      = 0.5 + 0.5 - 2*12/12 = -1.0
    let m = Mesh {
        v_new: vec![0.0, 0.0, 6.0],
        ..mesh()
    };
    let rows = set_up_heat_boundary(
        HeatLevel::Second,
        HeatBoundaryInput {
            k_top_t: -1,
            cw: 2.0,
            ..input(&m)
        },
    )
    .unwrap();
    assert_coeffs(rows.top, Some(-1.5), Some(-1.0), None, None);
}

#[test]
fn top_boundary_kind_zero_is_rejected() {
    let m = mesh();
    let err = set_up_heat_boundary(
        HeatLevel::First,
        HeatBoundaryInput {
            k_top_t: 0,
            ..input(&m)
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        HeatBoundaryError::TopBoundaryKindNotSupported { code: 0 }
    );
}

#[test]
fn sinusoidal_top_is_rejected() {
    let m = mesh();
    let err = set_up_heat_boundary(
        HeatLevel::First,
        HeatBoundaryInput {
            t_period: 1.0,
            ..input(&m)
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        HeatBoundaryError::SinusoidalTopNotSupported { period: 1.0 }
    );
}

#[test]
fn single_node_is_rejected() {
    let m = Mesh {
        x: vec![0.0],
        temp_old: vec![10.0],
        v_old: vec![0.0],
        v_new: vec![0.0],
        cap: vec![1.0],
        cond: vec![1.0],
        sink: vec![0.0],
    };
    let err = set_up_heat_boundary(HeatLevel::First, input(&m)).unwrap_err();
    assert_eq!(err, HeatBoundaryError::TooFewNodes { count: 1 });
}

#[test]
fn length_mismatch_is_rejected() {
    let m = Mesh {
        cap: vec![1.0; 2],
        ..mesh()
    };
    let err = set_up_heat_boundary(HeatLevel::First, input(&m)).unwrap_err();
    assert_eq!(
        err,
        HeatBoundaryError::LengthMismatch {
            name: "cap",
            expected: 3,
            actual: 2
        }
    );
}

#[test]
fn non_finite_input_is_rejected() {
    let m = Mesh {
        temp_old: vec![10.0, f64::NAN, 30.0],
        ..mesh()
    };
    let err = set_up_heat_boundary(HeatLevel::First, input(&m)).unwrap_err();
    assert_eq!(
        err,
        HeatBoundaryError::NonFiniteInput {
            name: "temp_old",
            index: 1
        }
    );
}

#[test]
fn non_positive_time_step_is_rejected() {
    let m = mesh();
    let err = set_up_heat_boundary(
        HeatLevel::Second,
        HeatBoundaryInput {
            dt: 0.0,
            ..input(&m)
        },
    )
    .unwrap_err();
    assert_eq!(err, HeatBoundaryError::NonPositiveTimeStep { value: 0.0 });
}

#[test]
fn non_positive_boundary_spacing_is_rejected() {
    // Bottom spacing x[1]-x[0] = 0.
    let m = Mesh {
        x: vec![0.0, 0.0, 2.0],
        ..mesh()
    };
    let err = set_up_heat_boundary(HeatLevel::First, input(&m)).unwrap_err();
    assert_eq!(
        err,
        HeatBoundaryError::NonPositiveSpacing { location: "bottom" }
    );
    // Top spacing x[2]-x[1] = 0.
    let m = Mesh {
        x: vec![0.0, 1.0, 1.0],
        ..mesh()
    };
    let err = set_up_heat_boundary(HeatLevel::First, input(&m)).unwrap_err();
    assert_eq!(
        err,
        HeatBoundaryError::NonPositiveSpacing { location: "top" }
    );
}

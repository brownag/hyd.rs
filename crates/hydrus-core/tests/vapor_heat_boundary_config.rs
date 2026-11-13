use hydrus_core::heat::config::{HeatConfig, HeatConfigError, HeatConfigInput};
use hydrus_core::heat::vapor::{VaporConfig, VaporConfigInput};

fn heat(l_vapor: bool, k_top_t: i32, k_bot_t: i32) -> HeatConfigInput {
    HeatConfigInput {
        l_vapor,
        l_en_bal: false,
        i_dual_por: 0,
        l_w_dep: false,
        i_model: 0,
        domain_count: 1,
        k_top_t,
        t_period: 0.0,
        k_bot_t,
        campbell: false,
    }
}

fn vapor_input(h: HeatConfigInput) -> VaporConfigInput {
    VaporConfigInput {
        heat: h,
        water_enabled: true,
        heat_enabled: true,
        i_enhanc: 1,
        hysteresis: 0,
        chemistry: false,
        density_coupling: false,
        centrifugal: false,
        top_temperature: 20.0,
        bottom_temperature: 20.0,
        x_conv: 1.0,
        t_conv: 1.0,
    }
}

/// Audited candidate forms: DF, DZ, FD, FF, FZ (top code, bottom code).
const ACCEPTED: [(i32, i32); 5] = [(1, -1), (1, 0), (-1, 1), (-1, -1), (-1, 0)];

#[test]
fn audited_candidate_forms_are_accepted_with_vapor() {
    for (top, bot) in ACCEPTED {
        let cfg = HeatConfig::new(heat(true, top, bot))
            .unwrap_or_else(|e| panic!("vapor rejected ({top},{bot}): {e:?}"));
        assert!(cfg.vapor_enabled(), "({top},{bot})");
        VaporConfig::new(vapor_input(heat(true, top, bot)))
            .unwrap_or_else(|e| panic!("VaporConfig rejected ({top},{bot}): {e:?}"));
    }
}

#[test]
fn every_nonzero_bottom_code_is_accepted_with_vapor_top_minus_one() {
    for bot in [-1, -2, -7, 1, 5] {
        assert!(
            HeatConfig::new(heat(true, -1, bot)).is_ok(),
            "FF/FD bottom {bot} rejected"
        );
    }
}

#[test]
fn rejects_top_codes_other_than_minus_one_and_positive_with_vapor() {
    for top in [-2, -5, 0] {
        for bot in [1, -1, 0] {
            let err = HeatConfig::new(heat(true, top, bot))
                .expect_err("vapor accepted rejected top code");
            if top == 0 {
                assert!(matches!(
                    err,
                    HeatConfigError::TopBoundaryKindNotSupported { code: 0 }
                ));
            } else {
                assert_eq!(
                    err,
                    HeatConfigError::VaporSelectionNotSupported { name: "kTopT" },
                    "top {top}, bottom {bot}"
                );
            }
        }
    }
}

#[test]
fn vapor_rejections_for_campbell_and_sinusoidal_remain() {
    let mut h = heat(true, -1, 1);
    h.campbell = true;
    assert_eq!(
        HeatConfig::new(h).unwrap_err(),
        HeatConfigError::VaporSelectionNotSupported { name: "iCampbell" }
    );
    let mut h = heat(true, -1, 1);
    h.t_period = 1.0;
    assert!(matches!(
        HeatConfig::new(h).unwrap_err(),
        HeatConfigError::SinusoidalTopNotSupported { .. }
    ));
}

#[test]
fn heat_only_configs_with_the_same_codes_still_construct() {
    for (top, bot) in ACCEPTED {
        assert!(
            HeatConfig::new(heat(false, top, bot)).is_ok(),
            "heat-only ({top},{bot}) changed"
        );
    }
}

//! Heat-only configuration contract (`planning/master/M6_HEAT_CONTRACT.md`).
//!
//! Waiver: `HeatConfigError::BoundaryKindNotSupported` has no reject test here.
//! The sign mapping for `kTopT` and `kBotT` covers every nonzero code, and the
//! zero codes have their own rules, so no current input reaches the variant. It
//! is kept as a catch-all for future codes (approved as option (a), 24.2.1).

use hydrus_core::heat::config::{
    BottomBoundaryKind, HeatConfig, HeatConfigError, HeatConfigInput, TopBoundaryKind,
};

/// Heat-only, supported selection: Dirichlet top, Dirichlet bottom, default conductivity.
fn heat_only() -> HeatConfigInput {
    HeatConfigInput {
        l_vapor: false,
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

#[test]
fn accepts_heat_only_dirichlet_selection() {
    let config = HeatConfig::new(heat_only()).expect("supported selection");
    assert_eq!(config.top_boundary(), TopBoundaryKind::Dirichlet);
    assert_eq!(config.bottom_boundary(), BottomBoundaryKind::Dirichlet);
    assert!(!config.campbell());
}

#[test]
fn accepts_flux_top_and_flux_bottom() {
    let config = HeatConfig::new(HeatConfigInput {
        k_top_t: -1,
        k_bot_t: -1,
        ..heat_only()
    })
    .expect("supported flux selection");
    assert_eq!(config.top_boundary(), TopBoundaryKind::Flux);
    assert_eq!(config.bottom_boundary(), BottomBoundaryKind::Flux);
}

#[test]
fn accepts_zero_gradient_bottom() {
    let config = HeatConfig::new(HeatConfigInput {
        k_bot_t: 0,
        ..heat_only()
    })
    .expect("zero-gradient bottom is supported");
    assert_eq!(config.bottom_boundary(), BottomBoundaryKind::ZeroGradient);
}

#[test]
fn accepts_campbell_conductivity_both_ways() {
    let default = HeatConfig::new(heat_only()).expect("iCampbell=0");
    assert!(!default.campbell());
    let campbell = HeatConfig::new(HeatConfigInput {
        campbell: true,
        ..heat_only()
    })
    .expect("iCampbell=1 is in scope (D13)");
    assert!(campbell.campbell());
}

#[test]
fn accepts_bounded_vapor_selection() {
    let config = HeatConfig::new(HeatConfigInput {
        l_vapor: true,
        ..heat_only()
    })
    .expect("bounded vapor heat selection");
    assert!(config.vapor_enabled());
    assert!(!config.campbell());
    assert_eq!(config.top_boundary(), TopBoundaryKind::Dirichlet);
    assert_eq!(config.bottom_boundary(), BottomBoundaryKind::Dirichlet);
}

#[test]
fn rejects_energy_balance() {
    let err = HeatConfig::new(HeatConfigInput {
        l_en_bal: true,
        ..heat_only()
    })
    .unwrap_err();
    assert!(matches!(
        err,
        HeatConfigError::EnergyBalanceNotSupported { value: true }
    ));
}

#[test]
fn rejects_dual_porosity() {
    let err = HeatConfig::new(HeatConfigInput {
        i_dual_por: 1,
        ..heat_only()
    })
    .unwrap_err();
    assert!(matches!(
        err,
        HeatConfigError::DualPorosityNotSupported { value: 1 }
    ));
}

#[test]
fn rejects_temperature_dependent_water() {
    let err = HeatConfig::new(HeatConfigInput {
        l_w_dep: true,
        ..heat_only()
    })
    .unwrap_err();
    assert!(matches!(
        err,
        HeatConfigError::TemperatureDependentWaterNotSupported { value: true }
    ));
}

#[test]
fn rejects_non_van_genuchten_model() {
    let err = HeatConfig::new(HeatConfigInput {
        i_model: 1,
        ..heat_only()
    })
    .unwrap_err();
    assert!(matches!(
        err,
        HeatConfigError::HydraulicModelNotSupported { selector: 1 }
    ));
}

#[test]
fn rejects_multi_domain() {
    let err = HeatConfig::new(HeatConfigInput {
        domain_count: 2,
        ..heat_only()
    })
    .unwrap_err();
    assert!(matches!(
        err,
        HeatConfigError::DomainCountNotSupported { count: 2 }
    ));
}

#[test]
fn rejects_zero_top_boundary_kind() {
    let err = HeatConfig::new(HeatConfigInput {
        k_top_t: 0,
        ..heat_only()
    })
    .unwrap_err();
    assert!(matches!(
        err,
        HeatConfigError::TopBoundaryKindNotSupported { code: 0 }
    ));
}

#[test]
fn rejects_sinusoidal_top_boundary() {
    let err = HeatConfig::new(HeatConfigInput {
        t_period: 24.0,
        ..heat_only()
    })
    .unwrap_err();
    assert!(matches!(
        err,
        HeatConfigError::SinusoidalTopNotSupported { t_period } if t_period == 24.0
    ));
}

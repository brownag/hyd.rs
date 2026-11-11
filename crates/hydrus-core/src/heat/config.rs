//! Heat-only configuration contract (`planning/master/M6_HEAT_CONTRACT.md`,
//! `## Supported` and `## Rejected`).
//!
//! Every rejected combination returns a typed [`HeatConfigError`] carrying the
//! offending value. Nothing is silently approximated.

use std::fmt;

/// Typed rejection for configurations outside the M6 heat-only selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HeatConfigError {
    /// `lVapor = true`: vapor terms are out of scope (`## Rejected`).
    VaporNotSupported { value: bool },
    /// `lEnBal = true` after `MeteoIn`: energy balance is out of scope.
    EnergyBalanceNotSupported { value: bool },
    /// `iDualPor != 0`: dual porosity is out of scope.
    DualPorosityNotSupported { value: i32 },
    /// `lWDep = true`: heat-to-water feedback is forbidden by D9.
    TemperatureDependentWaterNotSupported { value: bool },
    /// `iModel != 0`: only van Genuchten is supported (same rule as `WaterProductConfig`).
    HydraulicModelNotSupported { selector: i32 },
    /// Domain count other than one (same rule as `WaterProductConfig`).
    DomainCountNotSupported { count: usize },
    /// `kTopT == 0`: `SetUpHeat` leaves node N unassigned in the source.
    TopBoundaryKindNotSupported { code: i32 },
    /// `tPeriod > 0`: sinusoidal top boundary is not in the M6 selection.
    SinusoidalTopNotSupported { t_period: f64 },
    /// Catch-all for a boundary code outside the Supported sign classes.
    /// Kept for future codes; the current sign mapping cannot reach it.
    BoundaryKindNotSupported { code: i32 },
}

impl fmt::Display for HeatConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VaporNotSupported { value } => {
                write!(formatter, "heat configuration rejects lVapor={value}")
            }
            Self::EnergyBalanceNotSupported { value } => {
                write!(formatter, "heat configuration rejects lEnBal={value}")
            }
            Self::DualPorosityNotSupported { value } => {
                write!(formatter, "heat configuration rejects iDualPor={value}")
            }
            Self::TemperatureDependentWaterNotSupported { value } => {
                write!(formatter, "heat configuration rejects lWDep={value}")
            }
            Self::HydraulicModelNotSupported { selector } => {
                write!(formatter, "heat configuration rejects iModel={selector}")
            }
            Self::DomainCountNotSupported { count } => {
                write!(formatter, "heat configuration rejects domain count {count}")
            }
            Self::TopBoundaryKindNotSupported { code } => {
                write!(formatter, "heat configuration rejects kTopT={code}")
            }
            Self::SinusoidalTopNotSupported { t_period } => {
                write!(formatter, "heat configuration rejects tPeriod={t_period}")
            }
            Self::BoundaryKindNotSupported { code } => {
                write!(
                    formatter,
                    "heat configuration rejects boundary kind code {code}"
                )
            }
        }
    }
}

impl std::error::Error for HeatConfigError {}

/// Top boundary kinds supported in M6, by source sign class (`SetUpHeat`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopBoundaryKind {
    /// `kTopT > 0`: constant temperature.
    Dirichlet,
    /// `kTopT < 0`: heat flux from water.
    Flux,
}

/// Bottom boundary kinds supported in M6, by source sign class (`SetUpHeat`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BottomBoundaryKind {
    /// `kBotT > 0`: constant temperature.
    Dirichlet,
    /// `kBotT < 0`: heat flux from water.
    Flux,
    /// `kBotT == 0`: zero temperature gradient, `T(1) = T(2)`.
    ZeroGradient,
}

/// Raw heat-only selection as read from input. The caller supplies source values.
///
/// The caller maps `iCampbell` to `campbell` (0 is false, 1 is true) and must
/// reject any other integer value itself. `l_en_bal` is the effective value
/// after `MeteoIn` (`HYDRUS.FOR:244`, `INPUT.FOR:966`), not the header value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatConfigInput {
    pub l_vapor: bool,
    /// Effective `lEnBal`, after `MeteoIn`.
    pub l_en_bal: bool,
    pub i_dual_por: i32,
    pub l_w_dep: bool,
    pub i_model: i32,
    pub domain_count: usize,
    pub k_top_t: i32,
    pub t_period: f64,
    pub k_bot_t: i32,
    /// `iCampbell == 1` (true) or `iCampbell == 0` (false).
    pub campbell: bool,
}

/// Validated heat-only configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeatConfig {
    top: TopBoundaryKind,
    bottom: BottomBoundaryKind,
    campbell: bool,
}

impl HeatConfig {
    /// Accepts only the M6 heat-only selection in `## Supported`.
    pub fn new(input: HeatConfigInput) -> Result<Self, HeatConfigError> {
        if input.l_vapor {
            return Err(HeatConfigError::VaporNotSupported {
                value: input.l_vapor,
            });
        }
        if input.l_en_bal {
            return Err(HeatConfigError::EnergyBalanceNotSupported {
                value: input.l_en_bal,
            });
        }
        if input.i_dual_por != 0 {
            return Err(HeatConfigError::DualPorosityNotSupported {
                value: input.i_dual_por,
            });
        }
        if input.l_w_dep {
            return Err(HeatConfigError::TemperatureDependentWaterNotSupported {
                value: input.l_w_dep,
            });
        }
        if input.i_model != 0 {
            return Err(HeatConfigError::HydraulicModelNotSupported {
                selector: input.i_model,
            });
        }
        if input.domain_count != 1 {
            return Err(HeatConfigError::DomainCountNotSupported {
                count: input.domain_count,
            });
        }
        let top = match input.k_top_t {
            0 => {
                return Err(HeatConfigError::TopBoundaryKindNotSupported {
                    code: input.k_top_t,
                })
            }
            code if code > 0 => TopBoundaryKind::Dirichlet,
            _ => TopBoundaryKind::Flux,
        };
        if input.t_period > 0.0 {
            return Err(HeatConfigError::SinusoidalTopNotSupported {
                t_period: input.t_period,
            });
        }
        let bottom = match input.k_bot_t {
            0 => BottomBoundaryKind::ZeroGradient,
            code if code > 0 => BottomBoundaryKind::Dirichlet,
            _ => BottomBoundaryKind::Flux,
        };
        Ok(Self {
            top,
            bottom,
            campbell: input.campbell,
        })
    }

    pub fn top_boundary(self) -> TopBoundaryKind {
        self.top
    }

    pub fn bottom_boundary(self) -> BottomBoundaryKind {
        self.bottom
    }

    /// True when the Campbell conductivity branch (`iCampbell = 1`) is selected.
    pub fn campbell(self) -> bool {
        self.campbell
    }
}

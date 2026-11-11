//! Heat-only accepted-step energy accounting: storage change, boundary heat and sink heat.
//! Storage follows the `Cap` and `dx` weights of `Temper` and `SetUpHeat` (`src_reference/TEMPER.FOR`).
//! Contract: planning/master/M6_HEAT_CONTRACT.md. No energy-balance terms (`lEnBal`) are included.

use std::fmt;

/// Caller-supplied accepted-step inputs. Slices hold one entry per node, `N >= 3`.
///
/// The balance derives no boundary or sink values. The caller (driver, 24.4.1) supplies
/// `boundary_in` (top `heat_flux` from `HeatTransportReport`), `boundary_out` (bottom and any
/// conductive end fluxes computed from the solved state) and `sink_heat`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatBalanceInput<'a> {
    /// Node coordinates `x`.
    pub x: &'a [f64],
    /// Volumetric heat capacity from `ThOld` (`TempCap`).
    pub cap_old: &'a [f64],
    /// Temperature at the start of the step (`TempO`).
    pub temp_old: &'a [f64],
    /// Volumetric heat capacity from `ThNew` (`TempCap`).
    pub cap_new: &'a [f64],
    /// Temperature of the accepted step (`TempN`).
    pub temp_new: &'a [f64],
    /// Heat entering at the top, positive inward.
    pub boundary_in: f64,
    /// Heat leaving at the bottom or any conductive end, positive outward.
    pub boundary_out: f64,
    /// Heat removed by sinks over the step, positive for removal.
    pub sink_heat: f64,
}

/// Per-step balance terms. `absolute_residual = storage_change - (boundary_in - boundary_out - sink)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatBalanceReport {
    /// `S(new) - S(old)` with `S = sum_i w_i * Cap_i * T_i`.
    pub storage_change: f64,
    pub boundary_in: f64,
    pub boundary_out: f64,
    pub sink: f64,
    pub absolute_residual: f64,
}

/// Typed rejections for [`HeatBalance::record_step`].
#[derive(Clone, Debug, PartialEq)]
pub enum HeatBalanceError {
    TooFewNodes {
        count: usize,
    },
    LengthMismatch {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
    NonFinite {
        name: &'static str,
    },
}

impl fmt::Display for HeatBalanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewNodes { count } => {
                write!(
                    formatter,
                    "heat balance needs at least 3 nodes, got {count}"
                )
            }
            Self::LengthMismatch {
                name,
                expected,
                actual,
            } => write!(
                formatter,
                "heat balance {name} has length {actual}, expected {expected}"
            ),
            Self::NonFinite { name } => write!(formatter, "heat balance {name} is not finite"),
        }
    }
}

impl std::error::Error for HeatBalanceError {}

/// Accepted-only cumulative heat accounting, mirroring `WaterAccounting` (`water_state.rs:18-24`).
/// Changed only by a successful [`HeatBalance::record_step`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeatBalance {
    pub accepted_steps: u64,
    pub cumulative_storage_change: f64,
    /// Sum of `boundary_in - boundary_out`.
    pub cumulative_boundary_flux: f64,
    pub cumulative_sink: f64,
}

impl HeatBalance {
    /// Validates the inputs, computes the step balance, and adds it to the accumulators only on success.
    /// A rejected call leaves `self` unchanged.
    pub fn record_step(
        &mut self,
        input: HeatBalanceInput<'_>,
    ) -> Result<HeatBalanceReport, HeatBalanceError> {
        let n = input.x.len();
        if n < 3 {
            return Err(HeatBalanceError::TooFewNodes { count: n });
        }
        for (name, actual) in [
            ("cap_old", input.cap_old.len()),
            ("temp_old", input.temp_old.len()),
            ("cap_new", input.cap_new.len()),
            ("temp_new", input.temp_new.len()),
        ] {
            if actual != n {
                return Err(HeatBalanceError::LengthMismatch {
                    name,
                    expected: n,
                    actual,
                });
            }
        }
        for (name, values) in [
            ("x", input.x),
            ("cap_old", input.cap_old),
            ("temp_old", input.temp_old),
            ("cap_new", input.cap_new),
            ("temp_new", input.temp_new),
        ] {
            if values.iter().any(|value| !value.is_finite()) {
                return Err(HeatBalanceError::NonFinite { name });
            }
        }
        for (name, value) in [
            ("boundary_in", input.boundary_in),
            ("boundary_out", input.boundary_out),
            ("sink_heat", input.sink_heat),
        ] {
            if !value.is_finite() {
                return Err(HeatBalanceError::NonFinite { name });
            }
        }

        let storage_change = storage_integral(input.x, input.cap_new, input.temp_new)
            - storage_integral(input.x, input.cap_old, input.temp_old);
        let flux_net = input.boundary_in - input.boundary_out - input.sink_heat;
        let absolute_residual = storage_change - flux_net;
        if !storage_change.is_finite() {
            return Err(HeatBalanceError::NonFinite {
                name: "storage_change",
            });
        }
        if !absolute_residual.is_finite() {
            return Err(HeatBalanceError::NonFinite {
                name: "absolute_residual",
            });
        }

        self.accepted_steps += 1;
        self.cumulative_storage_change += storage_change;
        self.cumulative_boundary_flux += input.boundary_in - input.boundary_out;
        self.cumulative_sink += input.sink_heat;
        Ok(HeatBalanceReport {
            storage_change,
            boundary_in: input.boundary_in,
            boundary_out: input.boundary_out,
            sink: input.sink_heat,
            absolute_residual,
        })
    }
}

/// `sum_i w_i * cap_i * temp_i` with half-cell end weights and `(x[i+1]-x[i-1])/2` interior weights.
/// Inputs are validated by the caller (`n >= 3`, equal lengths, finite).
fn storage_integral(x: &[f64], cap: &[f64], temp: &[f64]) -> f64 {
    let last = x.len() - 1;
    let weight = |i: usize| {
        if i == 0 {
            (x[1] - x[0]) / 2.0
        } else if i == last {
            (x[last] - x[last - 1]) / 2.0
        } else {
            (x[i + 1] - x[i - 1]) / 2.0
        }
    };
    cap.iter()
        .zip(temp)
        .enumerate()
        .map(|(i, (cap, temp))| weight(i) * cap * temp)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    const X: [f64; 3] = [0.0, 1.0, 2.0];

    #[test]
    fn non_finite_input_is_rejected_and_leaves_state_unchanged() {
        let cap = [2.0; 3];
        let temp = [10.0; 3];
        let mut bad_temp = temp;
        bad_temp[1] = f64::NAN;
        let cases = [
            (bad_temp, 0.0, 0.0, 0.0, "temp_new"),
            (temp, f64::INFINITY, 0.0, 0.0, "boundary_in"),
            (temp, 0.0, 0.0, f64::NAN, "sink_heat"),
        ];
        for (temp_new, boundary_in, boundary_out, sink_heat, name) in cases {
            let mut balance = HeatBalance::default();
            let before = balance;
            let error = balance
                .record_step(HeatBalanceInput {
                    x: &X,
                    cap_old: &cap,
                    temp_old: &temp,
                    cap_new: &cap,
                    temp_new: &temp_new,
                    boundary_in,
                    boundary_out,
                    sink_heat,
                })
                .unwrap_err();
            assert_eq!(error, HeatBalanceError::NonFinite { name });
            assert_eq!(balance, before);
        }
    }

    #[test]
    fn zero_flux_closed_column_has_zero_storage_change_and_residual() {
        let cap = [2.0; 3];
        let temp = [10.0; 3];
        let mut balance = HeatBalance::default();
        let report = balance
            .record_step(HeatBalanceInput {
                x: &X,
                cap_old: &cap,
                temp_old: &temp,
                cap_new: &cap,
                temp_new: &temp,
                boundary_in: 0.0,
                boundary_out: 0.0,
                sink_heat: 0.0,
            })
            .unwrap();
        assert_eq!(report.storage_change, 0.0);
        assert_eq!(report.absolute_residual, 0.0);
        assert_eq!(balance.accepted_steps, 1);
    }

    #[test]
    fn known_flux_step_matches_hand_computed_storage_and_residual() {
        // Weights w = [0.5, 1.0, 0.5], Cap = 2 everywhere.
        // S_old = 2 * (0.5*0 + 1*0 + 0.5*10) = 10. S_new = 2 * (0.5*0 + 1*2 + 0.5*10) = 14.
        // dS = 4. Net flux = 5 - 1 - 0.5 = 3.5. Residual = 4 - 3.5 = 0.5.
        let cap = [2.0; 3];
        let temp_old = [0.0, 0.0, 10.0];
        let temp_new = [0.0, 2.0, 10.0];
        let mut balance = HeatBalance::default();
        let report = balance
            .record_step(HeatBalanceInput {
                x: &X,
                cap_old: &cap,
                temp_old: &temp_old,
                cap_new: &cap,
                temp_new: &temp_new,
                boundary_in: 5.0,
                boundary_out: 1.0,
                sink_heat: 0.5,
            })
            .unwrap();
        assert_eq!(report.storage_change, 4.0);
        assert_eq!(report.absolute_residual, 0.5);
        assert_eq!(balance.accepted_steps, 1);
        assert_eq!(balance.cumulative_storage_change, 4.0);
        assert_eq!(balance.cumulative_boundary_flux, 4.0);
        assert_eq!(balance.cumulative_sink, 0.5);
    }
}

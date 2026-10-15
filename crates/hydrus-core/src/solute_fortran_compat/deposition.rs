//! Source `Blocking` (`SOLUTE.FOR:1561-1594`) and `Deposit` (`1595-1660`) kernels, in
//! default REAL (`f32`). iPsi selection, `Dc`/`aa` provenance and level selection belong
//! to the Coeff port (18.9.4); MassTran stays in `accounting.rs`. `Exclusion` is not
//! covered here.

use super::types::KernelError;

/// Source `Blocking`: returns `psi`, starting from `psi=1`. iPsi 3 is active (its
/// continuation lines are column-6 statements) and has no `SMax` guard, so `SMax==0`
/// yields IEEE inf/NaN exactly as the source arithmetic would.
pub(crate) fn blocking(i_psi: i32, smax: f32, ss: f32, x: f32, dc: f32, smax2: f32) -> f32 {
    let mut psi = 1.0_f32;
    match i_psi {
        1 if smax > 0.0 => {
            psi = 1.0 - ss / smax;
        }
        2 if smax > 0.0 => {
            psi = ss.powf(smax).max(psi);
        }
        3 => {
            let binf = 1.0 / smax;
            let sinf = 0.546_f32;
            let minf = dc;
            let c = sinf * binf * ss;
            if ss <= 0.8 * smax {
                psi = 1.0 - 4.0 * c + 3.08 * c.powf(2.0) + 1.4069 * c.powf(3.0);
            }
            if ss > 0.8 * smax {
                psi = (1.0 - binf * ss).powf(3.0) / (2.0 * minf.powf(2.0) * binf.powf(3.0));
            }
        }
        4 if smax > 0.0 && dc > 0.0 => {
            psi = ((x.abs() + dc) / dc).powf(-smax);
        }
        5 => {
            if smax > 0.0 && dc > 0.0 {
                psi = ((x.abs() + dc) / dc).powf(-smax);
            }
            if smax2 > 0.0 {
                psi *= 1.0 - ss / smax2;
            }
        }
        _ => {}
    }
    psi
}

/// Inputs to source `Deposit`; `alfa*` are the sticking efficiencies (`Alfa1/2`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct DepositInput {
    pub alfa1: f32,
    pub alfa2: f32,
    pub dc1: f32,
    pub dp1: f32,
    pub theta: f32,
    pub q: f32,
    pub temp: f32,
    pub x_conv: f32,
    pub t_conv: f32,
}

/// Source `Deposit`, returning `(Ka1, Ka2)`. The source's interactive `STOP` when both
/// diameters are nonpositive is an `Err`.
// The source's approximate PI literal is intentional.
#[allow(clippy::approx_constant)]
pub(crate) fn deposit(i: &DepositInput) -> Result<(f32, f32), KernelError> {
    let dc = i.dc1 / i.x_conv;
    let dp = i.dp1 / i.x_conv;
    if dp <= 0.0 && dc <= 0.0 {
        return Err("source Deposit stops when both particle and grain diameters are nonpositive");
    }
    // Exponent rule: the source's INTEGER exponents (`gamma**5`, `gamma**6`, `Dp**2`) are
    // repeated multiplications in gfortran, so they use `powi`; real-literal exponents
    // (`**(1./3.)`, `**1.2`, and `**2.` in Blocking) stay `powf`.
    let pi = 3.1415_f32;
    let mu = 0.00093_f32;
    let bk = 1.38048e-23_f32;
    let h = 1.0e-20_f32;
    let g = 9.81_f32;
    let rop = 1080.0_f32;
    let rof = 998.0_f32;
    let veloc = (i.q / i.x_conv * i.t_conv).abs();
    let pveloc = veloc / i.theta;
    let (e_diff, e_inter, e_grav);
    if veloc > 0.0 {
        let gamma = (1.0 - i.theta).powf(1.0 / 3.0);
        let a_s = 2.0 * (1.0 - gamma.powi(5))
            / (2.0 - 3.0 * gamma + 3.0 * gamma.powi(5) - 2.0 * gamma.powi(6));
        let n_pe = 3.0 * pi * mu * dp * dc * veloc / (bk * (i.temp + 273.15));
        e_diff = 4.0 * a_s.powf(1.0 / 3.0) * n_pe.powf(-2.0 / 3.0);
        let n_lo = 4.0 * h / (9.0 * pi * mu * dp.powi(2) * veloc);
        let n_r = dp / dc;
        e_inter = a_s * n_lo.powf(1.0 / 8.0) * n_r.powf(15.0 / 8.0);
        let n_g = g * (rop - rof) * dp.powi(2) / (18.0 * mu * veloc);
        e_grav = 0.00338 * a_s * n_g.powf(1.2) * n_r.powf(-0.4);
    } else {
        e_diff = 0.0;
        e_inter = 0.0;
        e_grav = 0.0;
    }
    let eta = e_diff + e_inter + e_grav;
    let ka1 = 3.0 * (1.0 - i.theta) / 2.0 / dc * eta * i.alfa1 * pveloc / i.t_conv;
    let ka2 = 3.0 * (1.0 - i.theta) / 2.0 / dc * eta * i.alfa2 * pveloc / i.t_conv;
    Ok((ka1, ka2))
}

/// Nodewise `Deposit` inputs. `theta` is the mobile water content the caller already
/// reduced by `ThImob`; `dc1`, `dp1`, `alfa1`, `alfa2` are the caller's resolved values.
pub(crate) struct DepositNodes<'a> {
    pub alfa1: f32,
    pub alfa2: f32,
    pub dc1: f32,
    pub dp1: f32,
    pub theta: &'a [f32],
    pub velocity: &'a [f32],
    pub temperature: &'a [f32],
    pub x_conv: f32,
    pub t_conv: f32,
}

impl DepositNodes<'_> {
    pub(crate) fn validate(&self) -> Result<(), KernelError> {
        let n = self.theta.len();
        if n == 0 || self.velocity.len() != n || self.temperature.len() != n {
            return Err("Deposit node arrays must be nonempty and have matching lengths");
        }
        let scalars = [
            self.alfa1,
            self.alfa2,
            self.dc1,
            self.dp1,
            self.x_conv,
            self.t_conv,
        ];
        if scalars.iter().any(|v| !v.is_finite()) || self.x_conv == 0.0 || self.t_conv == 0.0 {
            return Err("Deposit scalars must be finite with nonzero unit conversions");
        }
        if self.theta.iter().any(|&t| !t.is_finite() || t <= 0.0)
            || self.velocity.iter().any(|v| !v.is_finite())
            || self.temperature.iter().any(|v| !v.is_finite())
        {
            return Err("Deposit node values must be finite with positive water content");
        }
        Ok(())
    }
}

/// Per-node `(rKa1, rKa2)`, in node order, for `SorbConcState::deposited_ka`.
pub(crate) fn deposit_nodes(nodes: &DepositNodes<'_>) -> Result<Vec<(f32, f32)>, KernelError> {
    nodes.validate()?;
    (0..nodes.theta.len())
        .map(|k| {
            deposit(&DepositInput {
                alfa1: nodes.alfa1,
                alfa2: nodes.alfa2,
                dc1: nodes.dc1,
                dp1: nodes.dp1,
                theta: nodes.theta[k],
                q: nodes.velocity[k],
                temp: nodes.temperature[k],
                x_conv: nodes.x_conv,
                t_conv: nodes.t_conv,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5 * b.abs().max(1.0)
    }
    fn input(q: f32) -> DepositInput {
        DepositInput {
            alfa1: 0.1,
            alfa2: 0.2,
            dc1: 0.0005,
            dp1: 0.00000095,
            theta: 0.4,
            q,
            temp: 20.0,
            x_conv: 1.0,
            t_conv: 1.0,
        }
    }

    // These pins repeat the values accounting.rs's own test pins for its private
    // copy, so both implementations are held to the same numbers.
    #[test]
    fn blocking_matches_accounting_pinned_values() {
        assert_eq!(blocking(1, 10.0, 2.0, 0.0, 1.0, 0.0), 0.8);
        assert_eq!(blocking(2, 2.0, 0.5, 0.0, 1.0, 0.0), 1.0);
        assert!((blocking(3, 10.0, 1.0, 0.0, 1.0, 0.0) - 0.791_011).abs() < 1e-5);
        assert_eq!(blocking(4, 1.0, 0.0, 1.0, 1.0, 0.0), 0.5);
        assert_eq!(blocking(5, 1.0, 2.0, 1.0, 1.0, 4.0), 0.25);
    }

    #[test]
    fn blocking_guards_and_default() {
        assert_eq!(blocking(0, 5.0, 1.0, 1.0, 1.0, 1.0), 1.0);
        assert_eq!(blocking(9, 5.0, 1.0, 1.0, 1.0, 1.0), 1.0);
        assert_eq!(blocking(1, 0.0, 1.0, 0.0, 1.0, 0.0), 1.0);
        assert_eq!(blocking(2, 0.0, 3.0, 0.0, 1.0, 0.0), 1.0);
        assert_eq!(blocking(2, 2.0, 3.0, 0.0, 1.0, 0.0), 9.0);
        assert_eq!(blocking(4, 0.0, 0.0, 1.0, 1.0, 0.0), 1.0);
        assert_eq!(blocking(4, 1.0, 0.0, 1.0, 0.0, 0.0), 1.0);
        assert_eq!(blocking(5, 0.0, 2.0, 1.0, 1.0, 4.0), 0.5);
        assert_eq!(blocking(5, 1.0, 2.0, 1.0, 1.0, 0.0), 0.5);
    }

    #[test]
    fn blocking_type3_both_branches_and_zero_smax() {
        // ss > 0.8*SMax: Binf=1, (1-0.9)^3 / (2*Minf^2*1) with Minf=2
        let hi = blocking(3, 1.0, 0.9, 0.0, 2.0, 0.0);
        assert!(close(hi, 0.001 / 8.0), "{hi}");
        // ss == 0: polynomial branch gives exactly 1
        assert_eq!(blocking(3, 10.0, 0.0, 0.0, 1.0, 0.0), 1.0);
        // SMax==0: Binf=inf, so the result is not finite
        assert!(!blocking(3, 0.0, 0.0, 0.0, 1.0, 0.0).is_finite());
        assert!(!blocking(3, 0.0, 1.0, 0.0, 1.0, 0.0).is_finite());
    }

    #[test]
    fn deposit_zero_velocity_gives_zero() {
        assert_eq!(deposit(&input(0.0)).unwrap(), (0.0, 0.0));
    }

    #[test]
    fn deposit_stop_error() {
        let mut i = input(0.01);
        i.dc1 = 0.0;
        i.dp1 = 0.0;
        assert!(deposit(&i).is_err());
    }

    #[test]
    fn deposit_hand_calculated_value() {
        // Independent f64 evaluation of the source equations (source PI literal).
        let (k1, k2) = deposit(&input(0.01)).unwrap();
        assert!(
            ((k1 - 0.001_737_506_5) / 0.001_737_506_5).abs() < 2e-3,
            "{k1}"
        );
        assert!(((k2 - 0.003_475_013) / 0.003_475_013).abs() < 2e-3, "{k2}");
        assert_eq!(deposit(&input(-0.01)).unwrap(), (k1, k2));
    }

    #[test]
    fn deposit_nodes_in_order_and_validated() {
        let nodes = DepositNodes {
            alfa1: 0.1,
            alfa2: 0.2,
            dc1: 0.0005,
            dp1: 0.00000095,
            theta: &[0.4, 0.4],
            velocity: &[0.01, 0.0],
            temperature: &[20.0, 20.0],
            x_conv: 1.0,
            t_conv: 1.0,
        };
        let out = deposit_nodes(&nodes).unwrap();
        assert_eq!(out[0], deposit(&input(0.01)).unwrap());
        assert_eq!(out[1], (0.0, 0.0));
        let bad = DepositNodes {
            velocity: &[0.01],
            ..nodes
        };
        assert!(deposit_nodes(&bad).is_err());
    }
}

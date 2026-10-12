//! Interior `MatSet` rows and `BanSol` solver from `SOLUTE.FOR`.
//!
//! Endpoint row construction is intentionally a named integration seam for
//! task 18.5.2. `g0`/`g1` are likewise caller-provided source-term seams for
//! process modules; this module does not derive reaction or sorption terms.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssemblyLevel {
    Old,
    New,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EndpointAssemblySeam {
    /// Previously accumulated RHS contributions, including endpoint terms.
    /// Interior assembly leaves the endpoint rows and RHS contributions alone.
    pub rhs: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InteriorRows {
    /// Double-precision source `B` (lower diagonal), `D` (diagonal), `E`
    /// (upper diagonal), and `F` arrays.
    pub lower: Vec<f64>,
    pub diagonal: Vec<f64>,
    pub upper: Vec<f64>,
    pub rhs: Vec<f64>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AssemblyInputs<'a> {
    pub x: &'a [f32],
    pub theta_old: &'a [f32],
    pub theta_new: &'a [f32],
    pub velocity_old: &'a [f32],
    pub velocity_new: &'a [f32],
    pub concentration: &'a [f32],
    pub dispersion: &'a [f32],
    pub retardation: &'a [f32],
    pub upstream_weight: &'a [f32],
    /// Process contribution seams, populated by the relevant process module.
    pub g0: &'a [f32],
    pub g1: &'a [f32],
}

/// Assemble interior nodes. `!` at physical column 6 in SOLUTE.FOR:836-849
/// is a fixed-form continuation marker, so Level-1 F(i) is executable.
pub(crate) fn assemble_interior(
    inputs: AssemblyInputs<'_>,
    level: AssemblyLevel,
    epsi: f32,
    alf: f32,
    dt: f32,
    endpoint_seam: EndpointAssemblySeam,
) -> Result<InteriorRows, &'static str> {
    let n = inputs.x.len();
    if n < 3
        || inputs.theta_old.len() != n
        || inputs.theta_new.len() != n
        || inputs.velocity_old.len() != n
        || inputs.velocity_new.len() != n
        || inputs.concentration.len() != n
        || inputs.dispersion.len() != n
        || inputs.retardation.len() != n
        || inputs.upstream_weight.len() != n
        || inputs.g0.len() != n
        || inputs.g1.len() != n
        || endpoint_seam.rhs.len() != n
    {
        return Err("interior assembly arrays must have the same length of at least three");
    }
    let mut result = InteriorRows {
        lower: vec![0.0; n],
        diagonal: vec![0.0; n],
        upper: vec![0.0; n],
        rhs: endpoint_seam.rhs,
    };
    for i in 1..n - 1 {
        let a1 = inputs.x[i] - inputs.x[i - 1];
        let b1 = inputs.x[i + 1] - inputs.x[i];
        let dx = (inputs.x[i + 1] - inputs.x[i - 1]) / 2.0;
        if level == AssemblyLevel::Old {
            let th = inputs.theta_old;
            let v = inputs.velocity_old;
            let c = inputs.concentration;
            let d = inputs.dispersion;
            // Independent terms follow the complete source continuation chain (lines 835-849).
            let left = c[i - 1]
                * alf
                * ((th[i - 1] * d[i - 1] + th[i] * d[i]) / a1 / 2.0
                    + ((2.0 + 3.0 * inputs.upstream_weight[i - 1]) * v[i - 1] + v[i]) / 6.0
                    + a1 / 12.0 * (inputs.g1[i - 1] + inputs.g1[i]));
            let center = c[i]
                * (dx / dt * th[i] * inputs.retardation[i]
                    + alf
                        * (-(th[i - 1] * d[i - 1] + th[i] * d[i]) / a1 / 2.0
                            - (th[i + 1] * d[i + 1] + th[i] * d[i]) / b1 / 2.0
                            - (v[i + 1]
                                + 3.0
                                    * (inputs.upstream_weight[i - 1]
                                        + inputs.upstream_weight[i])
                                    * v[i]
                                - v[i - 1])
                                / 6.0
                            + (a1 * (inputs.g1[i - 1] + 3.0 * inputs.g1[i])
                                + b1 * (3.0 * inputs.g1[i] + inputs.g1[i + 1]))
                                / 12.0));
            let right = c[i + 1]
                * alf
                * ((th[i + 1] * d[i + 1] + th[i] * d[i]) / b1 / 2.0
                    - (v[i] + (2.0 - 3.0 * inputs.upstream_weight[i]) * v[i + 1]) / 6.0
                    + b1 / 12.0 * (inputs.g1[i] + inputs.g1[i + 1]));
            let source = alf
                * (a1 * (inputs.g0[i - 1] + 2.0 * inputs.g0[i])
                    + b1 * (2.0 * inputs.g0[i] + inputs.g0[i + 1]))
                / 6.0;
            result.rhs[i] = (((left + center) + right) + source) as f64;
        } else {
            let th = inputs.theta_new;
            let v = inputs.velocity_new;
            let disp = inputs.dispersion;
            let g1 = inputs.g1;
            let lower = epsi
                * (-(th[i - 1] * disp[i - 1] + th[i] * disp[i]) / a1 / 2.0
                    - ((2.0 + 3.0 * inputs.upstream_weight[i - 1]) * v[i - 1] + v[i]) / 6.0
                    - a1 / 12.0 * (g1[i - 1] + g1[i]));
            let diagonal = dx / dt * th[i] * inputs.retardation[i]
                + epsi
                    * ((th[i - 1] * disp[i - 1] + th[i] * disp[i]) / a1 / 2.0
                        + (th[i + 1] * disp[i + 1] + th[i] * disp[i]) / b1 / 2.0
                        + (v[i + 1]
                            + 3.0
                                * (inputs.upstream_weight[i - 1] + inputs.upstream_weight[i])
                                * v[i]
                            - v[i - 1])
                            / 6.0
                        - (a1 * (g1[i - 1] + 3.0 * g1[i]) + b1 * (3.0 * g1[i] + g1[i + 1])) / 12.0);
            let upper = epsi
                * (-(th[i + 1] * disp[i + 1] + th[i] * disp[i]) / b1 / 2.0
                    + (v[i] + (2.0 - 3.0 * inputs.upstream_weight[i]) * v[i + 1]) / 6.0
                    - b1 / 12.0 * (g1[i] + g1[i + 1]));
            let source = epsi
                * (a1 * (inputs.g0[i - 1] + 2.0 * inputs.g0[i])
                    + b1 * (2.0 * inputs.g0[i] + inputs.g0[i + 1]))
                / 6.0;
            // Fortran evaluates each coefficient expression in default REAL,
            // then stores it in DOUBLE PRECISION. F(i) is already double, so
            // its existing value is added to the promoted source contribution.
            result.lower[i] = lower as f64;
            result.diagonal[i] = diagonal as f64;
            result.upper[i] = upper as f64;
            result.rhs[i] += source as f64;
        }
    }
    Ok(result)
}

/// Source `BanSol`: forward elimination in ascending index order followed by
/// back-substitution in descending index order. There is deliberately no
/// pivoting or diagonal repair.
pub(crate) fn ban_solve(
    lower: &mut [f64],
    diagonal: &mut [f64],
    upper: &mut [f64],
    rhs: &mut [f64],
) -> Result<(), &'static str> {
    let n = diagonal.len();
    if n == 0 || lower.len() != n || upper.len() != n || rhs.len() != n {
        return Err("tridiagonal arrays must have the same nonzero length");
    }
    for i in 1..n {
        diagonal[i] = diagonal[i] - lower[i] * upper[i - 1] / diagonal[i - 1];
        rhs[i] = rhs[i] - lower[i] * rhs[i - 1] / diagonal[i - 1];
    }
    rhs[n - 1] /= diagonal[n - 1];
    for i in 1..n {
        let j = n - i - 1;
        rhs[j] = (rhs[j] - upper[j] * rhs[j + 1]) / diagonal[j];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> AssemblyInputs<'static> {
        AssemblyInputs {
            x: &[0.0, 1.0, 3.0],
            theta_old: &[0.25, 0.35, 0.45],
            theta_new: &[0.25, 0.35, 0.45],
            velocity_old: &[0.15, 0.25, 0.35],
            velocity_new: &[0.15, 0.25, 0.35],
            concentration: &[2.0, 3.0, 5.0],
            dispersion: &[0.01, 0.02, 0.03],
            retardation: &[1.0, 2.0, 3.0],
            upstream_weight: &[0.1, 0.2, 0.3],
            g0: &[0.4, 0.5, 0.6],
            g1: &[-0.1, -0.2, -0.3],
        }
    }

    #[test]
    fn new_level_assembles_source_order_interior_coefficients_and_rhs() {
        let result = assemble_interior(
            inputs(),
            AssemblyLevel::New,
            0.5,
            0.7,
            0.25,
            EndpointAssemblySeam { rhs: vec![7.0; 3] },
        )
        .unwrap();
        let expected_lower = 0.5_f32
            * (-(0.25 * 0.01 + 0.35 * 0.02) / 1.0 / 2.0
                - ((2.0 + 3.0 * 0.1) * 0.15 + 0.25) / 6.0
                - 1.0 / 12.0 * (-0.1 - 0.2));
        let expected_upper = 0.5_f32
            * (-(0.45 * 0.03 + 0.35 * 0.02) / 2.0 / 2.0 + (0.25 + (2.0 - 3.0 * 0.2) * 0.35) / 6.0
                - 2.0 / 12.0 * (-0.2 - 0.3));
        let expected_rhs =
            7.0_f64 + (0.5_f32 * (1.0 * (0.4 + 2.0 * 0.5) + 2.0 * (2.0 * 0.5 + 0.6)) / 6.0) as f64;
        assert_eq!(result.lower[1], expected_lower as f64);
        assert_eq!(result.upper[1], expected_upper as f64);
        assert_eq!(result.rhs[1], expected_rhs);
        assert_eq!(result.rhs[0], 7.0);
        assert_eq!(result.rhs[2], 7.0);
    }

    #[test]
    fn level_one_assembles_complete_fixed_form_rhs_continuation_chain() {
        let p = inputs();
        let result = assemble_interior(
            p,
            AssemblyLevel::Old,
            0.6,
            0.7,
            0.5,
            EndpointAssemblySeam {
                rhs: vec![1.0, 2.0, 3.0],
            },
        )
        .unwrap();
        let a = 1.0_f32;
        let b = 2.0_f32;
        let dx = 1.5_f32;
        // Independent scalar sequencing from source lines 835-849 (fixed-form
        // continuation records at physical column 6): each source arithmetic
        // operation is rounded to f32 before the next operation.
        let left_flux =
            ((p.theta_old[0] * p.dispersion[0] + p.theta_old[1] * p.dispersion[1]) / a) / 2.0;
        let left_flux = left_flux
            + ((2.0 + 3.0 * p.upstream_weight[0]) * p.velocity_old[0] + p.velocity_old[1]) / 6.0;
        let left_flux = left_flux + a / 12.0 * (p.g1[0] + p.g1[1]);
        let left = (p.concentration[0] * 0.7_f32) * left_flux;
        let mass = ((dx / 0.5_f32) * p.theta_old[1]) * p.retardation[1];
        let center_flux =
            -((p.theta_old[0] * p.dispersion[0] + p.theta_old[1] * p.dispersion[1]) / a) / 2.0;
        let center_flux = center_flux
            - ((p.theta_old[2] * p.dispersion[2] + p.theta_old[1] * p.dispersion[1]) / b) / 2.0;
        let center_flux = center_flux
            - (p.velocity_old[2]
                + 3.0 * (p.upstream_weight[0] + p.upstream_weight[1]) * p.velocity_old[1]
                - p.velocity_old[0])
                / 6.0;
        let center_flux =
            center_flux + (a * (p.g1[0] + 3.0 * p.g1[1]) + b * (3.0 * p.g1[1] + p.g1[2])) / 12.0;
        let center = p.concentration[1] * (mass + 0.7_f32 * center_flux);
        let right_flux =
            ((p.theta_old[2] * p.dispersion[2] + p.theta_old[1] * p.dispersion[1]) / b) / 2.0;
        let right_flux = right_flux
            - (p.velocity_old[1] + (2.0 - 3.0 * p.upstream_weight[1]) * p.velocity_old[2]) / 6.0;
        let right_flux = right_flux + b / 12.0 * (p.g1[1] + p.g1[2]);
        let right = (p.concentration[2] * 0.7_f32) * right_flux;
        let g0_sum = a * (p.g0[0] + 2.0 * p.g0[1]) + b * (2.0 * p.g0[1] + p.g0[2]);
        let source = (0.7_f32 * g0_sum) / 6.0;
        let expected_rhs = (((left + center) + right) + source) as f64;
        assert_eq!(result.rhs[1], expected_rhs);
        assert_ne!(result.rhs[1], 2.0);
        assert_eq!([result.rhs[0], result.rhs[2]], [1.0, 3.0]);
        assert_eq!(result.diagonal, vec![0.0; 3]);
    }

    #[test]
    fn source_ban_solve_eliminates_and_back_substitutes_in_order() {
        let mut lower = vec![0.0, -1.0, -1.0];
        let mut diagonal = vec![2.0, 2.0, 2.0];
        let mut upper = vec![-1.0, -1.0, 0.0];
        let mut rhs = vec![1.0, 0.0, 1.0];
        ban_solve(&mut lower, &mut diagonal, &mut upper, &mut rhs).unwrap();
        for value in rhs {
            assert!((value - 1.0).abs() < 1.0e-14);
        }
    }

    #[test]
    fn rejects_mismatched_arrays_without_mutation() {
        let mut lower = vec![0.0, 1.0];
        let mut diagonal = vec![2.0];
        let mut upper = vec![0.0];
        let mut rhs = vec![1.0];
        assert!(ban_solve(&mut lower, &mut diagonal, &mut upper, &mut rhs).is_err());
        assert_eq!(lower, vec![0.0, 1.0]);
        assert_eq!(diagonal, vec![2.0]);
        assert_eq!(rhs, vec![1.0]);
    }
}

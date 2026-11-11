//! Source-ordered transport coefficients from `SOLUTE.FOR` (`Coeff`, `Disper`,
//! and `PeCour`). Arithmetic here intentionally uses `f32`, the pinned build's
//! default REAL kind. Process contributions are inputs/seams, not implemented
//! by this module.

/// Test-only switch for the bounded Robin coefficient audit. It is absent from
/// non-test builds and defaults to the source-parity expression.
#[cfg(test)]
pub(crate) mod diagnostic {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    static STABLE_WEIGHTS: AtomicBool = AtomicBool::new(false);
    static FORCE_UPSTREAM_WEIGHT: AtomicBool = AtomicBool::new(false);
    static WEIGHT_CALLS: AtomicUsize = AtomicUsize::new(0);

    pub(crate) fn set_stable_weights(value: bool) {
        STABLE_WEIGHTS.store(value, Ordering::SeqCst);
    }

    pub(crate) fn stable_weights() -> bool {
        STABLE_WEIGHTS.load(Ordering::SeqCst)
    }

    pub(crate) fn set_force_upstream_weight(value: bool) {
        FORCE_UPSTREAM_WEIGHT.store(value, Ordering::SeqCst);
    }

    pub(crate) fn force_upstream_weight() -> bool {
        FORCE_UPSTREAM_WEIGHT.load(Ordering::SeqCst)
    }

    pub(crate) fn reset_weight_calls() {
        WEIGHT_CALLS.store(0, Ordering::SeqCst);
    }

    pub(crate) fn record_weight_call() {
        WEIGHT_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn weight_calls() -> usize {
        WEIGHT_CALLS.load(Ordering::SeqCst)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TransportNode {
    pub water: f32,
    pub gas_water: f32,
    pub saturation_water: f32,
    pub immobile_water: f32,
    pub velocity: f32,
    pub dispersivity: f32,
    pub molecular_diffusion: f32,
    pub gas_diffusion: f32,
    pub henry: f32,
    pub bulk_density: f32,
    pub equilibrium_fraction: f32,
    /// `f_em` (`SOLUTE.FOR:371`): `1.` unless `lDualNEq`. Kept separate from `Frac`
    /// so `Retard` keeps the source grouping `ro*Frac*f_em*xKs*dConc` (580).
    pub f_em: f32,
    pub sorption_coefficient: f32,
    pub exponent: f32,
    pub sorption_nu: f32,
    pub concentration_midpoint: f32,
    pub concentration_derivative: f32,
    pub sorption_derivative: f32,
    /// Contribution `fi` from source process terms; zero for the base path.
    pub process_correction: f32,
    pub tortuosity: bool,
    /// `iTort == 0` selects the first source formula; any other value selects
    /// the second.
    pub tortuosity_method: i32,
    pub mobile_immobile: bool,
    pub bacteria: bool,
    pub dual_domain: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TransportCoefficients {
    pub retardation: f32,
    pub dispersion: f32,
    pub tau_water: f32,
    pub tau_gas: f32,
}

/// Evaluate the source `Retard` and `Disper` expressions. No process formula is
/// inferred: the concentration derivative and process correction remain
/// explicit source inputs.
pub(crate) fn coefficients(
    node: TransportNode,
    dt: f32,
    level_is_new: bool,
    artificial_dispersion: bool,
    upstream_weighting: bool,
    peclet_threshold: f32,
) -> TransportCoefficients {
    let mut tau_water = 1.0_f32;
    let mut tau_gas = 1.0_f32;
    if node.tortuosity {
        let mut theta_s = node.saturation_water;
        if (node.mobile_immobile && !node.dual_domain) || node.bacteria {
            // SOLUTE.FOR:1036-1038 is a conditional `then` followed by the
            // continued assignment at column 6, and only the dual-domain
            // branch overwrites it.
            theta_s = (theta_s - node.immobile_water).max(0.001);
        }
        if node.dual_domain {
            theta_s = node.saturation_water + node.immobile_water;
        }
        if node.tortuosity_method == 0 {
            tau_water = node.water.powf(7.0 / 3.0) / theta_s.powi(2);
            tau_gas = node.gas_water.powf(7.0 / 3.0) / theta_s.powi(2);
        } else {
            tau_water = 0.66 * (node.water / theta_s).powf(8.0 / 3.0);
            tau_gas = node.gas_water.powf(1.5) / theta_s;
        }
    }

    // Retard at SOLUTE.FOR:580-592 forms the product and gas term before
    // division by water; preserve the default-REAL grouping.
    let retardation = (node.bulk_density
        * node.equilibrium_fraction
        * node.f_em
        * node.sorption_coefficient
        * node.concentration_derivative
        + node.gas_water * node.henry)
        / node.water
        + 1.0;
    // Disper at SOLUTE.FOR:1050 uses this left-to-right expression order.
    let mut dispersion = node.dispersivity * node.velocity.abs() / node.water
        + node.molecular_diffusion * tau_water
        + node.gas_water / node.water * node.gas_diffusion * node.henry * tau_gas;

    if !artificial_dispersion && !upstream_weighting {
        let mut fi = 0.0_f32;
        if node.concentration_midpoint > 0.0 {
            fi = 6.0
                * node.water
                * node.bulk_density
                * node.sorption_coefficient
                * node.concentration_midpoint.powf(node.exponent - 1.0)
                * (node.exponent
                    / (1.0 + node.sorption_nu * node.concentration_midpoint.powf(node.exponent))
                        .powi(2)
                    - 1.0
                        / (1.0
                            + node.sorption_nu * node.concentration_midpoint.powf(node.exponent)));
            fi += node.process_correction;
        }
        // Disper at SOLUTE.FOR:1052-1060; fi's continued formula at 1054-55
        // is executable when cMid > 0, as the `!` is in column 6.
        let denominator = 6.0
            * node.water
            * (node.water
                + node.bulk_density
                    * node.equilibrium_fraction
                    * node.sorption_coefficient
                    * node.sorption_derivative
                + node.gas_water * node.henry)
            + fi;
        let dpom = (dt / denominator).max(0.0);
        if level_is_new {
            dispersion = (dispersion - node.velocity * node.velocity * dpom).max(dispersion / 2.0);
        } else {
            dispersion += node.velocity * node.velocity * dpom;
        }
    }
    if artificial_dispersion {
        let mut dd = 0.0_f32;
        if peclet_threshold != 0.0 && node.velocity.abs() > 1.0e-15 {
            dd = node.velocity * node.velocity * dt
                / node.water
                / node.water
                / retardation
                / peclet_threshold;
        }
        if dd > dispersion {
            dispersion = dd;
        }
    }

    TransportCoefficients {
        retardation,
        dispersion,
        tau_water,
        tau_gas,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PecletInputs {
    pub dx: f32,
    pub dt: f32,
    pub velocity: f32,
    pub neighbor_velocity: f32,
    pub water: f32,
    pub neighbor_water: f32,
    pub dispersion: f32,
    pub neighbor_dispersion: f32,
    pub retardation: f32,
    pub neighbor_retardation: f32,
    pub upstream_weighting: bool,
    pub artificial_dispersion: bool,
    pub level_is_new: bool,
    pub iteration: usize,
    pub epsi: f32,
    pub threshold: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PecletResult {
    pub peclet_max: f32,
    pub courant_max: f32,
    pub dt_max: f32,
    pub weight: Option<f32>,
}

/// Source-ordered upstream weight at `SOLUTE.FOR:972` and the `wc` chain at
/// `1011-13`. The two f32 subtractions are intentionally preserved.
fn source_upstream_weight(pe2: f32) -> f32 {
    let tanh = (pe2.exp() - (-pe2).exp()) / (pe2.exp() + (-pe2).exp());
    (1.0 / tanh - 1.0 / pe2).clamp(-1.0, 1.0)
}

/// Source `PeCour` for a node with a forward neighbor. Callers aggregate the
/// maxima across nodes in the same descending source traversal.
pub(crate) fn pecour(input: PecletInputs) -> PecletResult {
    #[cfg(test)]
    let upstream_weighting = input.upstream_weighting || diagnostic::force_upstream_weight();
    #[cfg(not(test))]
    let upstream_weighting = input.upstream_weighting;
    let vv = if input.water > 1.0e-6 && input.neighbor_water > 1.0e-6 {
        (input.velocity.abs() / input.water + input.neighbor_velocity.abs() / input.neighbor_water)
            / 2.0
    } else {
        0.0
    };
    let vv1 = if input.water > 1.0e-6 && input.neighbor_water > 1.0e-6 {
        (input.velocity / input.water + input.neighbor_velocity / input.neighbor_water) / 2.0
    } else {
        0.0
    };
    let dd = (input.dispersion + input.neighbor_dispersion) / 2.0;
    let mut peclet = 99999.0_f32;
    let mut courant = 0.0_f32;
    let mut dt_max = 1.0e30_f32;
    let mut weight = None;
    if input.level_is_new {
        let v_max = (input.velocity.abs() + input.neighbor_velocity.abs())
            / (input.water + input.neighbor_water);
        let r_min = input.retardation.min(input.neighbor_retardation);
        if dd > 0.0 {
            peclet = vv.abs() * input.dx / dd;
        }
        courant = v_max * input.dt / input.dx / r_min;
        let mut courant_limit = 1.0_f32;
        if !upstream_weighting && !input.artificial_dispersion && peclet != 99999.0 {
            courant_limit = (input.threshold / 0.5_f32.max(peclet)).min(1.0);
        }
        if input.epsi < 1.0 && v_max > 1.0e-20 {
            dt_max = courant_limit * input.dx * r_min / v_max;
        }
    } else if upstream_weighting && input.iteration == 1 {
        #[cfg(test)]
        diagnostic::record_weight_call();
        let pe2 = if dd > 0.0 {
            input.dx * vv1 / dd / 2.0
        } else {
            11.0
        };
        let wc = if vv.abs() < 1.0e-30 {
            0.0
        } else if pe2.abs() > 10.0 {
            if vv1 > 0.0 {
                1.0
            } else if vv1 < 0.0 {
                -1.0
            } else {
                0.0
            }
        } else {
            #[cfg(test)]
            {
                if diagnostic::stable_weights() {
                    let pe = pe2 as f64;
                    (1.0 / pe.tanh() - 1.0 / pe).clamp(-1.0, 1.0) as f32
                } else {
                    source_upstream_weight(pe2)
                }
            }
            #[cfg(not(test))]
            {
                source_upstream_weight(pe2)
            }
        };
        weight = Some(wc);
    }
    PecletResult {
        peclet_max: peclet,
        courant_max: courant,
        dt_max,
        weight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2.0675669, bits from the reference script.
    const RETARD_F_EM: f32 = f32::from_bits(0x40045304);

    fn node() -> TransportNode {
        TransportNode {
            water: 0.3,
            gas_water: 0.1,
            saturation_water: 0.4,
            immobile_water: 0.0,
            velocity: 0.2,
            dispersivity: 0.01,
            molecular_diffusion: 0.02,
            gas_diffusion: 0.03,
            henry: 0.5,
            bulk_density: 1.2,
            equilibrium_fraction: 1.0,
            f_em: 1.0,
            sorption_coefficient: 0.5,
            exponent: 1.0,
            sorption_nu: 0.0,
            concentration_midpoint: 0.0,
            concentration_derivative: 1.0,
            sorption_derivative: 1.0,
            process_correction: 0.0,
            tortuosity: false,
            tortuosity_method: 0,
            mobile_immobile: false,
            bacteria: false,
            dual_domain: false,
        }
    }

    #[test]
    fn base_retardation_and_dispersion_use_default_real() {
        let result = coefficients(node(), 0.1, true, false, true, 0.1);
        assert_eq!(result.retardation, (0.6_f32 + 0.1 * 0.5) / 0.3 + 1.0);
        assert_eq!(
            result.dispersion,
            0.01 * 0.2 / 0.3 + 0.02 + 0.1 / 0.3 * 0.03 * 0.5
        );
    }

    #[test]
    fn tortuosity_methods_and_dual_saturation_are_distinct() {
        let mut input = node();
        input.tortuosity = true;
        let first = coefficients(input, 0.1, true, true, true, 0.1);
        assert_eq!(
            first.tau_water,
            input.water.powf(7.0 / 3.0) / input.saturation_water.powi(2)
        );
        input.tortuosity_method = 1;
        let second = coefficients(input, 0.1, true, true, true, 0.1);
        assert_eq!(
            second.tau_water,
            0.66 * (input.water / input.saturation_water).powf(8.0 / 3.0)
        );
        input.dual_domain = true;
        input.immobile_water = 0.2;
        let dual = coefficients(input, 0.1, true, true, true, 0.1);
        assert_eq!(dual.tau_gas, input.gas_water.powf(1.5) / 0.6);
    }

    #[test]
    fn non_artificial_correction_is_level_dependent_and_floored() {
        let input = node();
        let old = coefficients(input, 0.1, false, false, false, 0.1);
        let new = coefficients(input, 0.1, true, false, false, 0.1);
        assert!(old.dispersion > new.dispersion);
        assert!(new.dispersion >= (0.01 * 0.2 / 0.3 + 0.02 + 0.1 / 0.3 * 0.03 * 0.5) / 2.0);
    }

    #[test]
    fn f_em_keeps_the_source_product_grouping() {
        // Expected values: `.development/18.10.1/ref_retard.py` (independent f32
        // transcription of `Retard=(ro*Frac*f_em*xKs*dConc+ThG*Henry)/ThW+1.`).
        let mut n = node();
        n.bulk_density = 1.3;
        n.equilibrium_fraction = 0.7;
        n.f_em = 0.3;
        n.sorption_coefficient = 0.9;
        n.concentration_derivative = 1.1;
        let r = coefficients(n, 1.0, true, false, false, 0.0).retardation;
        assert_eq!(r.to_bits(), RETARD_F_EM.to_bits());
        // f_em = 1 is bit-identical to the pre-f_em product.
        let mut one = node();
        one.f_em = 1.0;
        let base = (one.bulk_density
            * one.equilibrium_fraction
            * one.sorption_coefficient
            * one.concentration_derivative
            + one.gas_water * one.henry)
            / one.water
            + 1.0;
        assert_eq!(
            coefficients(one, 1.0, true, false, false, 0.0)
                .retardation
                .to_bits(),
            base.to_bits()
        );
    }

    #[test]
    fn artificial_dispersion_and_weighting_select_separate_branches() {
        let mut input = node();
        input.velocity = 2.0;
        let base = coefficients(input, 1.0, true, false, false, 0.1);
        let artificial = coefficients(input, 1.0, true, true, false, 0.1);
        assert!(artificial.dispersion > base.dispersion);
        let both = coefficients(input, 1.0, true, true, true, 0.1);
        assert_eq!(both.dispersion, artificial.dispersion);
    }

    #[test]
    fn pecour_updates_weight_only_on_first_iteration_and_computes_limit() {
        let input = PecletInputs {
            dx: 1.0,
            dt: 0.1,
            velocity: 0.2,
            neighbor_velocity: 0.2,
            water: 0.3,
            neighbor_water: 0.3,
            dispersion: 0.1,
            neighbor_dispersion: 0.1,
            retardation: 2.0,
            neighbor_retardation: 2.0,
            upstream_weighting: true,
            artificial_dispersion: false,
            level_is_new: false,
            iteration: 1,
            epsi: 0.5,
            threshold: 0.2,
        };
        assert!(pecour(input).weight.unwrap().is_finite());
        assert_eq!(
            pecour(PecletInputs {
                iteration: 2,
                ..input
            })
            .weight,
            None
        );
        let new = pecour(PecletInputs {
            level_is_new: true,
            ..input
        });
        assert!(new.peclet_max.is_finite());
        assert!(new.courant_max > 0.0);
        assert!(new.dt_max > 0.0);
    }
}

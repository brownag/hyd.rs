use super::{HydraulicDerivatives, VanGenuchtenParameters};
use crate::HydraulicError;

const DRY_HEAD_FLOOR: f64 = -1.0e37;
const SOURCE_VALUE_FLOOR: f64 = 1.0e-37;

pub(super) struct ModelZeroEvaluation {
    pub(super) theta: f64,
    pub(super) conductivity: f64,
    pub(super) capacity: f64,
    pub(super) derivatives: HydraulicDerivatives,
}

pub(super) fn evaluate_model_zero(
    parameters: &VanGenuchtenParameters,
    head: f64,
) -> Result<ModelZeroEvaluation, HydraulicError> {
    if !head.is_finite() {
        return Err(HydraulicError::InvalidEvaluationDomain {
            name: "head",
            value: head,
        });
    }

    let delta = parameters.saturated_water_content - parameters.residual_water_content;
    if head >= 0.0 {
        return Ok(ModelZeroEvaluation {
            theta: parameters.saturated_water_content,
            conductivity: parameters.saturated_conductivity,
            capacity: 0.0,
            derivatives: HydraulicDerivatives {
                dtheta_dhead: 0.0,
                dconductivity_dhead: 0.0,
                dcapacity_dhead: 0.0,
            },
        });
    }

    let x = -parameters.alpha * head;
    let m = 1.0 - 1.0 / parameters.n; // MATERIAL.FOR: m=1.d0-1.d0/n.
    let a = 1.0 + x.powf(parameters.n);
    let se = a.powf(-m);
    let theta = parameters.residual_water_content + delta * se;
    let dse_dhead =
        m * parameters.n * parameters.alpha * x.powf(parameters.n - 1.0) * a.powf(-m - 1.0);
    let capacity = delta * dse_dhead;

    let root = se.sqrt();
    let inner = (1.0 - se.powf(1.0 / m)).max(0.0);
    let bend = 1.0 - inner.powf(m);
    let relative = se * root * bend * bend;
    // MATERIAL.FOR applies `sngl` at FK/FC/FQ/FH returns; this port keeps f64
    // throughout, while retaining the source's 1.e-37 floor semantics.
    let conductivity = (parameters.saturated_conductivity * relative).max(SOURCE_VALUE_FLOOR);

    let dcapacity_dhead = capacity
        * (-(parameters.n - 1.0) / x
            + (m + 1.0) * parameters.n * parameters.alpha * x.powf(parameters.n - 1.0) / a);
    let dconductivity_dhead = if parameters.saturated_conductivity == 0.0 {
        0.0
    } else {
        let d_bend_dse = inner.powf(m - 1.0) * se.powf(1.0 / m - 1.0);
        let d_relative_dse =
            relative * (1.5 / se + 2.0 * d_bend_dse / bend.max(SOURCE_VALUE_FLOOR));
        (parameters.saturated_conductivity * d_relative_dse * dse_dhead).max(0.0)
    };

    Ok(ModelZeroEvaluation {
        theta,
        conductivity,
        capacity,
        derivatives: HydraulicDerivatives {
            dtheta_dhead: capacity,
            dconductivity_dhead,
            dcapacity_dhead,
        },
    })
}

pub(super) fn inverse_head(
    parameters: &VanGenuchtenParameters,
    theta: f64,
) -> Result<f64, HydraulicError> {
    if !theta.is_finite() {
        return Err(HydraulicError::InvalidEvaluationDomain {
            name: "theta",
            value: theta,
        });
    }
    if theta < parameters.residual_water_content || theta > parameters.saturated_water_content {
        return Err(HydraulicError::InvalidEvaluationDomain {
            name: "theta",
            value: theta,
        });
    }
    if theta == parameters.saturated_water_content {
        return Ok(0.0);
    }
    if theta == parameters.residual_water_content {
        return Ok(DRY_HEAD_FLOOR);
    }
    let m = 1.0 - 1.0 / parameters.n;
    let se = (theta - parameters.residual_water_content)
        / (parameters.saturated_water_content - parameters.residual_water_content);
    Ok((-1.0 / parameters.alpha) * (se.powf(-1.0 / m) - 1.0).powf(1.0 / parameters.n))
}

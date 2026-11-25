//! Read-only `NOD_INF_V` observation view (`OUTPUT.FOR:760-816,891-926`).
//!
//! Source arrays and the Rust mesh are both bottom-first: source `i = 1` is the
//! bottom node and `i = N` the surface. `NodOut` loops `i = N..1` and prints node
//! `N - i + 1`, so Rust index `i` has output node `n - i`, and rows come out
//! top-first. Fluxes are recomputed from stored coefficients and the final
//! accepted heads/temperatures; they are not the solver velocities (`Veloc`),
//! which weight interior faces with the opposite-side spacing and add vapor
//! storage at both endpoints.

use super::driver::VaporStepReport;
use super::state::VaporState;
use super::VaporError;

/// Explicit inputs of one observation. Slices are bottom-first (Rust order).
#[derive(Clone, Copy, Debug)]
pub struct VaporObservationInput<'a> {
    pub time: f64,
    pub coordinates: &'a [f64],
    pub heads: &'a [f64],
    pub temperatures: &'a [f64],
    /// Stored `Con`, `ConLT`, `ConVh`, `ConVT`.
    pub liquid_conductivities: &'a [f64],
    pub liquid_thermal_conductivities: &'a [f64],
    pub vapor_head_conductivities: &'a [f64],
    pub vapor_thermal_conductivities: &'a [f64],
    /// Top-node liquid content `ThN(N)` and `ThOldT`. Equal for an initial print.
    pub top_liquid_new: f64,
    pub top_liquid_old: f64,
    /// Sink at the top node (`Sink(N)`).
    pub top_sink: f64,
    pub timestep: f64,
    /// `CosAlf` (with `fRE = 1`); applied to liquid flow only.
    pub gravity: f64,
}

/// One printed row, with both output-space and Rust-space identity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaporObservationRow {
    /// Printed node number (1 = surface).
    pub node: usize,
    /// Bottom-first mesh index of the same node.
    pub rust_index: usize,
    /// `x(i) - x(surface)`; zero at the top, negative below.
    pub depth: f64,
    pub con: f64,
    pub con_lt: f64,
    pub con_vh: f64,
    pub con_vt: f64,
    pub v_liquid: f64,
    pub v_vapor: f64,
    pub v_total: f64,
    pub v_vap_iso: f64,
    pub v_vap_term: f64,
}

/// Rows in printed (top-first) order at one time.
#[derive(Clone, Debug, PartialEq)]
pub struct VaporObservation {
    pub time: f64,
    pub rows: Vec<VaporObservationRow>,
}

fn len(name: &'static str, n: usize, v: &[f64]) -> Result<(), VaporError> {
    if v.len() != n {
        return Err(VaporError::LengthMismatch {
            name,
            expected: n,
            actual: v.len(),
        });
    }
    scalars(name, v)
}

fn scalars(name: &'static str, v: &[f64]) -> Result<(), VaporError> {
    match v.iter().position(|x| !x.is_finite()) {
        Some(i) => Err(VaporError::NonFinite {
            name,
            index: Some(i),
            value: v[i],
        }),
        None => Ok(()),
    }
}

/// Recompute the nine `NOD_INF_V` quantities as `NodOut` does.
pub fn observe_vapor(input: &VaporObservationInput<'_>) -> Result<VaporObservation, VaporError> {
    let x = input.coordinates;
    let n = x.len();
    if n < 3 {
        return Err(VaporError::TooFewNodes { count: n });
    }
    for (name, v) in [
        ("coordinates", x),
        ("heads", input.heads),
        ("temperatures", input.temperatures),
        ("liquid_conductivities", input.liquid_conductivities),
        (
            "liquid_thermal_conductivities",
            input.liquid_thermal_conductivities,
        ),
        ("vapor_head_conductivities", input.vapor_head_conductivities),
        (
            "vapor_thermal_conductivities",
            input.vapor_thermal_conductivities,
        ),
    ] {
        len(name, n, v)?;
    }
    scalars(
        "scalars",
        &[
            input.time,
            input.top_liquid_new,
            input.top_liquid_old,
            input.top_sink,
            input.timestep,
            input.gravity,
        ],
    )?;
    if input.timestep <= 0.0 {
        return Err(VaporError::InvalidValue {
            name: "timestep",
            value: input.timestep,
        });
    }
    if let Some(i) = x.windows(2).position(|p| p[1] <= p[0]) {
        return Err(VaporError::InvalidValue {
            name: "coordinates",
            value: x[i + 1],
        });
    }
    let (h, t) = (input.heads, input.temperatures);
    let (con, vh, vt) = (
        input.liquid_conductivities,
        input.vapor_head_conductivities,
        input.vapor_thermal_conductivities,
    );
    let avg = |a: &[f64], i: usize, j: usize| (a[i] + a[j]) / 2.0;
    let top = n - 1;
    let mut rows = Vec::with_capacity(n);
    for i in (0..n).rev() {
        let (liquid, iso, term);
        if i == 0 || i == top {
            let (a, b) = if i == 0 { (0, 1) } else { (top - 1, top) };
            let dx = x[b] - x[a];
            let base = -avg(con, a, b) * ((h[b] - h[a]) / dx + input.gravity);
            liquid = if i == top {
                base - (input.top_liquid_new - input.top_liquid_old) * dx / 2.0 / input.timestep
                    - input.top_sink * dx / 2.0
            } else {
                base
            };
            iso = -avg(vh, a, b) * (h[b] - h[a]) / dx;
            term = -avg(vt, a, b) * (t[b] - t[a]) / dx;
        } else {
            let dxa = x[i + 1] - x[i];
            let dxb = x[i] - x[i - 1];
            let w = dxa + dxb;
            let va = -avg(con, i, i + 1) * ((h[i + 1] - h[i]) / dxa + input.gravity);
            let vb = -avg(con, i, i - 1) * ((h[i] - h[i - 1]) / dxb + input.gravity);
            liquid = (va * dxa + vb * dxb) / w;
            let ia = -avg(vh, i, i + 1) * (h[i + 1] - h[i]) / dxa;
            let ib = -avg(vh, i, i - 1) * (h[i] - h[i - 1]) / dxb;
            iso = (ia * dxa + ib * dxb) / w;
            let ta = -avg(vt, i, i + 1) * (t[i + 1] - t[i]) / dxa;
            let tb = -avg(vt, i, i - 1) * (t[i] - t[i - 1]) / dxb;
            term = (ta * dxa + tb * dxb) / w;
        }
        let vapor = iso + term;
        rows.push(VaporObservationRow {
            node: n - i,
            rust_index: i,
            depth: x[i] - x[top],
            con: con[i],
            con_lt: input.liquid_thermal_conductivities[i],
            con_vh: vh[i],
            con_vt: vt[i],
            v_liquid: liquid,
            v_vapor: vapor,
            v_total: liquid + vapor,
            v_vap_iso: iso,
            v_vap_term: term,
        });
    }
    Ok(VaporObservation {
        time: input.time,
        rows,
    })
}

/// Observe the accepted state with the stored coefficients of its last step.
/// Reads only accepted heads/temperatures; neither argument is modified.
pub fn observe_step(
    state: &VaporState,
    report: &VaporStepReport,
    coordinates: &[f64],
    sink: &[f64],
    gravity: f64,
) -> Result<VaporObservation, VaporError> {
    let view = state.snapshot();
    let n = coordinates.len();
    len("coordinates", view.heads.len(), coordinates)?;
    if report.accepted_time != view.time {
        return Err(VaporError::InvalidValue {
            name: "step report does not match accepted state",
            value: report.accepted_time - view.time,
        });
    }
    if Some(report.timestep) != view.previous_timestep {
        return Err(VaporError::InvalidValue {
            name: "step report timestep does not match accepted state",
            value: report.timestep - view.previous_timestep.unwrap_or(f64::NAN),
        });
    }
    len("sink", n, sink)?;
    len("liquid_contents_old", n, &report.liquid_contents_old)?;
    len("liquid_contents_new", n, &report.liquid_contents_new)?;
    observe_vapor(&VaporObservationInput {
        time: view.time,
        coordinates,
        heads: view.heads,
        temperatures: view.temperatures,
        liquid_conductivities: &report.liquid_conductivities,
        liquid_thermal_conductivities: &report.liquid_thermal_conductivities,
        vapor_head_conductivities: &report.vapor_head_conductivities,
        vapor_thermal_conductivities: &report.vapor_thermal_conductivities,
        top_liquid_new: report.liquid_contents_new[n - 1],
        top_liquid_old: report.liquid_contents_old[n - 1],
        top_sink: sink[n - 1],
        timestep: report.timestep,
        gravity,
    })
}

/// Explicit inputs for `TLInf` top/bottom boundary fluxes (`OUTPUT.FOR:25-54`).
///
/// All slices are bottom-first (Rust mesh order: index `0` is the bottom node
/// `x(1)` and index `n - 1` is the surface node `x(N)`).
#[derive(Clone, Copy, Debug)]
pub struct TLevelFluxInput<'a> {
    pub coordinates: &'a [f64],
    pub heads: &'a [f64],
    pub temperatures: &'a [f64],
    /// Stored `Con`, `ConVh`, `ConVT` from the last Picard iteration.
    pub liquid_conductivities: &'a [f64],
    pub vapor_head_conductivities: &'a [f64],
    pub vapor_thermal_conductivities: &'a [f64],
    /// `ThOld`, `ThNew`, `ThVOld`, `ThVNew`.
    pub liquid_contents_old: &'a [f64],
    pub liquid_contents_new: &'a [f64],
    pub vapor_contents_old: &'a [f64],
    pub vapor_contents_new: &'a [f64],
    /// Root water uptake sink `Sink`.
    pub sink: &'a [f64],
    pub timestep: f64,
    /// `CosAlf` (with `fRE = 1`); applied to liquid flow only.
    pub gravity: f64,
}

/// Reported top and bottom boundary fluxes in the `TLInf` output form.
///
/// Positive fluxes are upward (out of the soil at the top, into the soil at the
/// bottom) on an upward-increasing `coordinates` axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TLevelFluxes {
    /// Total top boundary flux (`vTop` / `vT`).
    pub v_top: f64,
    /// Total bottom boundary flux (`vBot` / `vB`).
    pub v_bot: f64,
    /// Liquid component of the top boundary flux (`vTopW`).
    pub v_top_liquid: f64,
    /// Vapor component of the top boundary flux (`vTopV = vT - vTopW`).
    pub v_top_vapor: f64,
}

/// Compute `TLInf` top/bottom boundary fluxes (`OUTPUT.FOR:25-54`) in source order.
///
/// Evaluates `vT`, `vB`, `vTopW`, and `vTopV` with `fRE = 1` in `f64` precision
/// (ADR 0010, ADR 0015). The `ConLT` (`lWTDep`, rejected per G23), dual-porosity
/// (`iDualPor`), density (`lDensity`), and centrifuge (`lCentrif`) terms are
/// omitted because those selectors are rejected by [`super::VaporConfig`].
pub fn t_level_fluxes(input: &TLevelFluxInput<'_>) -> Result<TLevelFluxes, VaporError> {
    let x = input.coordinates;
    let n = x.len();
    if n < 3 {
        return Err(VaporError::TooFewNodes { count: n });
    }
    for (name, v) in [
        ("coordinates", x),
        ("heads", input.heads),
        ("temperatures", input.temperatures),
        ("liquid_conductivities", input.liquid_conductivities),
        ("vapor_head_conductivities", input.vapor_head_conductivities),
        (
            "vapor_thermal_conductivities",
            input.vapor_thermal_conductivities,
        ),
        ("liquid_contents_old", input.liquid_contents_old),
        ("liquid_contents_new", input.liquid_contents_new),
        ("vapor_contents_old", input.vapor_contents_old),
        ("vapor_contents_new", input.vapor_contents_new),
        ("sink", input.sink),
    ] {
        len(name, n, v)?;
    }
    scalars("scalars", &[input.timestep, input.gravity])?;
    if input.timestep <= 0.0 {
        return Err(VaporError::InvalidValue {
            name: "timestep",
            value: input.timestep,
        });
    }
    if let Some(i) = x.windows(2).position(|p| p[1] <= p[0]) {
        return Err(VaporError::InvalidValue {
            name: "coordinates",
            value: x[i + 1],
        });
    }

    let top = n - 1;
    let m = top - 1;
    let dt = input.timestep;
    let grav = input.gravity;
    let (h, t) = (input.heads, input.temperatures);
    let (con, con_vh, con_vt) = (
        input.liquid_conductivities,
        input.vapor_head_conductivities,
        input.vapor_thermal_conductivities,
    );
    let (th_old, th_new) = (input.liquid_contents_old, input.liquid_contents_new);
    let (th_v_old, th_v_new) = (input.vapor_contents_old, input.vapor_contents_new);
    let sink = input.sink;

    let dx_n = x[top] - x[m];
    let mut v_t = -(con[top] + con[m]) / 2.0 * ((h[top] - h[m]) / dx_n + grav)
        - (th_new[top] - th_old[top]) * dx_n / 2.0 / dt
        - sink[top] * dx_n / 2.0;
    let dx_1 = x[1] - x[0];
    let mut v_b = -(con[0] + con[1]) / 2.0 * ((h[1] - h[0]) / dx_1 + grav)
        + (th_new[0] - th_old[0]) * dx_1 / 2.0 / dt
        + sink[0] * dx_1 / 2.0;
    let v_top_w = v_t;
    v_t -= (con_vh[top] + con_vh[m]) / 2.0 * (h[top] - h[m]) / dx_n;
    v_b -= (con_vh[0] + con_vh[1]) / 2.0 * (h[1] - h[0]) / dx_1;
    v_t = v_t
        - (con_vt[top] + con_vt[m]) / 2.0 * (t[top] - t[m]) / dx_n
        - (th_v_new[top] - th_v_old[top]) * dx_n / 2.0 / dt;
    v_b = v_b - (con_vt[0] + con_vt[1]) / 2.0 * (t[1] - t[0]) / dx_1
        + (th_v_new[0] - th_v_old[0]) * dx_1 / 2.0 / dt;
    let v_top_v = v_t - v_top_w;

    Ok(TLevelFluxes {
        v_top: v_t,
        v_bot: v_b,
        v_top_liquid: v_top_w,
        v_top_vapor: v_top_v,
    })
}

/// Compute `TLInf` boundary fluxes for the accepted state using the stored
/// conductivities and water contents from its last step report.
pub fn t_level_fluxes_for_step(
    state: &VaporState,
    report: &VaporStepReport,
    coordinates: &[f64],
    sink: &[f64],
    gravity: f64,
) -> Result<TLevelFluxes, VaporError> {
    let view = state.snapshot();
    len("coordinates", view.heads.len(), coordinates)?;
    if report.accepted_time != view.time {
        return Err(VaporError::InvalidValue {
            name: "step report does not match accepted state",
            value: report.accepted_time - view.time,
        });
    }
    if Some(report.timestep) != view.previous_timestep {
        return Err(VaporError::InvalidValue {
            name: "step report timestep does not match accepted state",
            value: report.timestep - view.previous_timestep.unwrap_or(f64::NAN),
        });
    }
    t_level_fluxes(&TLevelFluxInput {
        coordinates,
        heads: view.heads,
        temperatures: view.temperatures,
        liquid_conductivities: &report.liquid_conductivities,
        vapor_head_conductivities: &report.vapor_head_conductivities,
        vapor_thermal_conductivities: &report.vapor_thermal_conductivities,
        liquid_contents_old: &report.liquid_contents_old,
        liquid_contents_new: &report.liquid_contents_new,
        vapor_contents_old: &report.vapor_contents_old,
        vapor_contents_new: &report.vapor_contents_new,
        sink,
        timestep: report.timestep,
        gravity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hand_t_level_fluxes(input: &TLevelFluxInput<'_>) -> TLevelFluxes {
        let n = input.coordinates.len();
        let top = n - 1;
        let m = n - 2;
        let dx_n = input.coordinates[top] - input.coordinates[m];
        let mut v_t = -(input.liquid_conductivities[top] + input.liquid_conductivities[m]) / 2.0
            * ((input.heads[top] - input.heads[m]) / dx_n + input.gravity)
            - (input.liquid_contents_new[top] - input.liquid_contents_old[top]) * dx_n
                / 2.0
                / input.timestep
            - input.sink[top] * dx_n / 2.0;
        let dx_1 = input.coordinates[1] - input.coordinates[0];
        let mut v_b = -(input.liquid_conductivities[0] + input.liquid_conductivities[1]) / 2.0
            * ((input.heads[1] - input.heads[0]) / dx_1 + input.gravity)
            + (input.liquid_contents_new[0] - input.liquid_contents_old[0]) * dx_1
                / 2.0
                / input.timestep
            + input.sink[0] * dx_1 / 2.0;
        let v_top_w = v_t;
        v_t -= (input.vapor_head_conductivities[top] + input.vapor_head_conductivities[m]) / 2.0
            * (input.heads[top] - input.heads[m])
            / dx_n;
        v_b -= (input.vapor_head_conductivities[0] + input.vapor_head_conductivities[1]) / 2.0
            * (input.heads[1] - input.heads[0])
            / dx_1;
        v_t = v_t
            - (input.vapor_thermal_conductivities[top] + input.vapor_thermal_conductivities[m])
                / 2.0
                * (input.temperatures[top] - input.temperatures[m])
                / dx_n
            - (input.vapor_contents_new[top] - input.vapor_contents_old[top]) * dx_n
                / 2.0
                / input.timestep;
        v_b = v_b
            - (input.vapor_thermal_conductivities[0] + input.vapor_thermal_conductivities[1]) / 2.0
                * (input.temperatures[1] - input.temperatures[0])
                / dx_1
            + (input.vapor_contents_new[0] - input.vapor_contents_old[0]) * dx_1
                / 2.0
                / input.timestep;
        let v_top_v = v_t - v_top_w;
        TLevelFluxes {
            v_top: v_t,
            v_bot: v_b,
            v_top_liquid: v_top_w,
            v_top_vapor: v_top_v,
        }
    }

    #[test]
    fn three_node_profile_matches_source_order_exact() {
        let input = TLevelFluxInput {
            coordinates: &[-10.0, -4.0, 0.0],
            heads: &[-120.0, -150.0, -210.0],
            temperatures: &[18.0, 21.5, 27.0],
            liquid_conductivities: &[0.08, 0.05, 0.02],
            vapor_head_conductivities: &[1.2e-5, 2.4e-5, 3.6e-5],
            vapor_thermal_conductivities: &[4.0e-4, 6.0e-4, 8.0e-4],
            liquid_contents_old: &[0.31, 0.28, 0.22],
            liquid_contents_new: &[0.305, 0.275, 0.21],
            vapor_contents_old: &[1.1e-4, 1.4e-4, 1.9e-4],
            vapor_contents_new: &[1.15e-4, 1.45e-4, 2.05e-4],
            sink: &[0.002, 0.003, 0.005],
            timestep: 0.25,
            gravity: 1.0,
        };
        let expected = hand_t_level_fluxes(&input);
        let actual = t_level_fluxes(&input).expect("valid 3-node profile");
        assert_eq!(actual, expected);
    }

    #[test]
    fn five_node_profile_matches_source_order_exact() {
        let input = TLevelFluxInput {
            coordinates: &[-20.0, -15.0, -9.0, -3.5, 0.0],
            heads: &[-80.0, -95.0, -130.0, -180.0, -260.0],
            temperatures: &[15.0, 16.5, 19.0, 23.0, 29.5],
            liquid_conductivities: &[0.12, 0.09, 0.06, 0.03, 0.015],
            vapor_head_conductivities: &[0.8e-5, 1.1e-5, 1.8e-5, 2.9e-5, 4.5e-5],
            vapor_thermal_conductivities: &[2.5e-4, 3.2e-4, 4.8e-4, 7.1e-4, 9.4e-4],
            liquid_contents_old: &[0.35, 0.33, 0.29, 0.25, 0.19],
            liquid_contents_new: &[0.348, 0.327, 0.286, 0.244, 0.181],
            vapor_contents_old: &[0.9e-4, 1.0e-4, 1.3e-4, 1.7e-4, 2.2e-4],
            vapor_contents_new: &[0.92e-4, 1.03e-4, 1.34e-4, 1.76e-4, 2.31e-4],
            sink: &[0.001, 0.002, 0.0025, 0.004, 0.006],
            timestep: 0.1,
            gravity: 1.0,
        };
        let expected = hand_t_level_fluxes(&input);
        let actual = t_level_fluxes(&input).expect("valid 5-node profile");
        assert_eq!(actual, expected);
    }

    #[test]
    fn zero_vapor_terms_give_pure_liquid_top_flux() {
        let input = TLevelFluxInput {
            coordinates: &[-10.0, -5.0, 0.0],
            heads: &[-100.0, -120.0, -160.0],
            temperatures: &[20.0, 22.0, 25.0],
            liquid_conductivities: &[0.04, 0.03, 0.02],
            vapor_head_conductivities: &[0.0, 0.0, 0.0],
            vapor_thermal_conductivities: &[0.0, 0.0, 0.0],
            liquid_contents_old: &[0.30, 0.28, 0.25],
            liquid_contents_new: &[0.298, 0.277, 0.245],
            vapor_contents_old: &[1.0e-4, 1.0e-4, 1.0e-4],
            vapor_contents_new: &[1.0e-4, 1.0e-4, 1.0e-4],
            sink: &[0.0, 0.0, 0.001],
            timestep: 0.5,
            gravity: 1.0,
        };
        let fluxes = t_level_fluxes(&input).expect("valid profile");
        assert_eq!(fluxes.v_top, fluxes.v_top_liquid);
        assert_eq!(fluxes.v_top_vapor, 0.0);
    }

    /// Because `TLInf` defines `vTopV = vT - vTopW` (exact Fortran order),
    /// reconstructing `v_top_liquid + v_top_vapor` in IEEE-754 `f64` matches
    /// `v_top` to within at most one rounding step (1 ULP of `v_top`).
    #[test]
    fn top_flux_components_sum_within_one_ulp() {
        let input = TLevelFluxInput {
            coordinates: &[-10.0, -4.0, 0.0],
            heads: &[-120.0, -150.0, -210.0],
            temperatures: &[18.0, 21.5, 27.0],
            liquid_conductivities: &[0.08, 0.05, 0.02],
            vapor_head_conductivities: &[1.2e-5, 2.4e-5, 3.6e-5],
            vapor_thermal_conductivities: &[4.0e-4, 6.0e-4, 8.0e-4],
            liquid_contents_old: &[0.31, 0.28, 0.22],
            liquid_contents_new: &[0.305, 0.275, 0.21],
            vapor_contents_old: &[1.1e-4, 1.4e-4, 1.9e-4],
            vapor_contents_new: &[1.15e-4, 1.45e-4, 2.05e-4],
            sink: &[0.002, 0.003, 0.005],
            timestep: 0.25,
            gravity: 1.0,
        };
        let fluxes = t_level_fluxes(&input).expect("valid profile");
        let sum = fluxes.v_top_liquid + fluxes.v_top_vapor;
        let ulp =
            (fluxes.v_top.next_up() - fluxes.v_top).max(fluxes.v_top - fluxes.v_top.next_down());
        assert!(
            (fluxes.v_top - sum).abs() <= ulp,
            "v_top={} vs v_top_liquid+v_top_vapor={} (diff={}, ulp={})",
            fluxes.v_top,
            sum,
            (fluxes.v_top - sum).abs(),
            ulp,
        );
    }

    #[test]
    fn validation_rejects_too_few_nodes_and_length_mismatch() {
        let x2 = [-5.0, 0.0];
        let v2 = [0.0, 0.0];
        let err = t_level_fluxes(&TLevelFluxInput {
            coordinates: &x2,
            heads: &v2,
            temperatures: &v2,
            liquid_conductivities: &v2,
            vapor_head_conductivities: &v2,
            vapor_thermal_conductivities: &v2,
            liquid_contents_old: &v2,
            liquid_contents_new: &v2,
            vapor_contents_old: &v2,
            vapor_contents_new: &v2,
            sink: &v2,
            timestep: 0.1,
            gravity: 1.0,
        })
        .unwrap_err();
        assert!(matches!(err, VaporError::TooFewNodes { count: 2 }));

        let x3 = [-10.0, -5.0, 0.0];
        let v3 = [0.0, 0.0, 0.0];
        let err = t_level_fluxes(&TLevelFluxInput {
            coordinates: &x3,
            heads: &v3,
            temperatures: &v3,
            liquid_conductivities: &v3,
            vapor_head_conductivities: &v3,
            vapor_thermal_conductivities: &v3,
            liquid_contents_old: &v3,
            liquid_contents_new: &v3,
            vapor_contents_old: &v3,
            vapor_contents_new: &v2,
            sink: &v3,
            timestep: 0.1,
            gravity: 1.0,
        })
        .unwrap_err();
        assert!(matches!(
            err,
            VaporError::LengthMismatch {
                name: "vapor_contents_new",
                expected: 3,
                actual: 2
            }
        ));

        let err = t_level_fluxes(&TLevelFluxInput {
            coordinates: &x3,
            heads: &v3,
            temperatures: &v3,
            liquid_conductivities: &v3,
            vapor_head_conductivities: &v3,
            vapor_thermal_conductivities: &v3,
            liquid_contents_old: &v3,
            liquid_contents_new: &v3,
            vapor_contents_old: &v3,
            vapor_contents_new: &v3,
            sink: &v3,
            timestep: 0.0,
            gravity: 1.0,
        })
        .unwrap_err();
        assert!(matches!(
            err,
            VaporError::InvalidValue {
                name: "timestep",
                ..
            }
        ));

        let bad_x = [-10.0, -10.0, 0.0];
        let err = t_level_fluxes(&TLevelFluxInput {
            coordinates: &bad_x,
            heads: &v3,
            temperatures: &v3,
            liquid_conductivities: &v3,
            vapor_head_conductivities: &v3,
            vapor_thermal_conductivities: &v3,
            liquid_contents_old: &v3,
            liquid_contents_new: &v3,
            vapor_contents_old: &v3,
            vapor_contents_new: &v3,
            sink: &v3,
            timestep: 0.1,
            gravity: 1.0,
        })
        .unwrap_err();
        assert!(matches!(
            err,
            VaporError::InvalidValue {
                name: "coordinates",
                ..
            }
        ));
    }
}

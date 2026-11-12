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

//! Heat boundary rows, ported from `SetUpHeat` (`src_reference/TEMPER.FOR:77-179`).
//! Contract: planning/master/M6_HEAT_CONTRACT.md (`## Supported`, `## Rejected`).

/// Predictor (`Level 1`, `ThOld`/`vOld`, assembles `F`) or corrector (`Level 2`, `ThNew`/`vNew`, assembles `B`, `D`, `E`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeatLevel {
    First,
    Second,
}

/// Boundary-row entries Fortran `B`, `D`, `E`, `F` at one end. `None` means
/// `SetUpHeat` leaves that entry untouched at this level.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoundaryCoefficients {
    pub b: Option<f64>,
    pub d: Option<f64>,
    pub e: Option<f64>,
    pub f: Option<f64>,
}

/// Output of [`set_up_heat_boundary`]: the bottom row (node 1) and top row (node N).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeatBoundaryRows {
    pub bottom: BoundaryCoefficients,
    pub top: BoundaryCoefficients,
}

/// Inputs to [`set_up_heat_boundary`]. Slices hold one entry per node, `N >= 2`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatBoundaryInput<'a> {
    /// `kTopT`: sign class selects Dirichlet (`>0`) or flux (`<0`). `0` is rejected.
    pub k_top_t: i32,
    /// `kBotT`: sign class selects Dirichlet (`>0`), flux (`<0`) or zero-gradient (`==0`).
    pub k_bot_t: i32,
    /// `tTopA`: top boundary temperature (equals `tTop` when `tPeriod <= 0`).
    pub t_top_a: f64,
    /// `tBot`: bottom boundary temperature.
    pub t_bottom: f64,
    /// `tPeriod`: sinusoidal period. Must be `<= 0`.
    pub t_period: f64,
    /// `HeatFl`: top heat flux, computed by the caller (`TEMPER.FOR:25-27`).
    pub heat_flux: f64,
    /// Time step `dt`.
    pub dt: f64,
    /// `Cw`: volumetric heat capacity of water.
    pub cw: f64,
    pub x: &'a [f64],
    /// `TempO`.
    pub temp_old: &'a [f64],
    /// `vOld`.
    pub v_old: &'a [f64],
    /// `vNew`.
    pub v_new: &'a [f64],
    /// `Cap`.
    pub cap: &'a [f64],
    /// `Cond`.
    pub cond: &'a [f64],
    /// `Sink`.
    pub sink: &'a [f64],
}

/// Typed rejections for [`set_up_heat_boundary`]. Each is raised instead of a Fortran stop or stale row.
#[derive(Clone, Debug, PartialEq)]
pub enum HeatBoundaryError {
    TooFewNodes {
        count: usize,
    },
    LengthMismatch {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
    NonFiniteInput {
        name: &'static str,
        index: usize,
    },
    NonPositiveTimeStep {
        value: f64,
    },
    NonPositiveSpacing {
        location: &'static str,
    },
    /// `kTopT == 0`: `SetUpHeat` has no branch, so the source row is stale.
    TopBoundaryKindNotSupported {
        code: i32,
    },
    SinusoidalTopNotSupported {
        period: f64,
    },
}

/// Computes the top and bottom boundary rows of `SetUpHeat` for one level.
///
/// Port of `TEMPER.FOR:77-179`, boundary branches only. Interior rows are not
/// assembled here; the caller owns them and the `HeatFl` input (`TEMPER.FOR:25-27`).
///
/// Integer codes are keyed on sign class, as the Fortran branches are, because the
/// integer code values are manual-pending in the contract. `kTopT`: `>0` Dirichlet,
/// `<0` flux, `0` rejected. `kBotT`: `>0` Dirichlet, `<0` flux, `==0` zero-gradient.
///
/// Level split (the contract, not a choice): Level 1 assembles `F` only, using
/// `TempO`, `vOld` and `HeatFl`. Level 2 assembles `B`, `D`, `E` only, using `vNew`.
/// Dirichlet rows are written at both levels. Entries the Fortran leaves untouched
/// at a level are `None`. 24.3.1 must apply only the `Some` entries.
///
/// Precision: the port is `f64` (D8). Each `SetUpHeat` expression is REAL in the
/// source; the same operation order is kept, and single-precision rounding is not
/// reproduced. Deviation from the bare Fortran: the source has no input checks
/// (`N >= 2`, finite inputs, `dt > 0`, positive boundary spacing). These are added as
/// typed errors, and the `dt > 0` guard applies to every kind, Dirichlet included.
pub fn set_up_heat_boundary(
    level: HeatLevel,
    input: HeatBoundaryInput<'_>,
) -> Result<HeatBoundaryRows, HeatBoundaryError> {
    let HeatBoundaryInput {
        k_top_t,
        k_bot_t,
        t_top_a,
        t_bottom,
        t_period,
        heat_flux,
        dt,
        cw,
        x,
        temp_old,
        v_old,
        v_new,
        cap,
        cond,
        sink,
    } = input;
    let n = x.len();
    if n < 2 {
        return Err(HeatBoundaryError::TooFewNodes { count: n });
    }
    for (name, actual) in [
        ("temp_old", temp_old.len()),
        ("v_old", v_old.len()),
        ("v_new", v_new.len()),
        ("cap", cap.len()),
        ("cond", cond.len()),
        ("sink", sink.len()),
    ] {
        if actual != n {
            return Err(HeatBoundaryError::LengthMismatch {
                name,
                expected: n,
                actual,
            });
        }
    }
    for (name, slice) in [
        ("x", x),
        ("temp_old", temp_old),
        ("v_old", v_old),
        ("v_new", v_new),
        ("cap", cap),
        ("cond", cond),
        ("sink", sink),
    ] {
        if let Some(index) = slice.iter().position(|value| !value.is_finite()) {
            return Err(HeatBoundaryError::NonFiniteInput { name, index });
        }
    }
    for (name, value) in [
        ("t_top_a", t_top_a),
        ("t_bottom", t_bottom),
        ("t_period", t_period),
        ("heat_flux", heat_flux),
        ("dt", dt),
        ("cw", cw),
    ] {
        if !value.is_finite() {
            return Err(HeatBoundaryError::NonFiniteInput { name, index: 0 });
        }
    }
    if dt <= 0.0 {
        return Err(HeatBoundaryError::NonPositiveTimeStep { value: dt });
    }
    if t_period > 0.0 {
        return Err(HeatBoundaryError::SinusoidalTopNotSupported { period: t_period });
    }
    if k_top_t == 0 {
        return Err(HeatBoundaryError::TopBoundaryKindNotSupported { code: k_top_t });
    }
    if x[1] - x[0] <= 0.0 {
        return Err(HeatBoundaryError::NonPositiveSpacing { location: "bottom" });
    }
    if x[n - 1] - x[n - 2] <= 0.0 {
        return Err(HeatBoundaryError::NonPositiveSpacing { location: "top" });
    }

    let level_two = level == HeatLevel::Second;
    let mut bottom = BoundaryCoefficients::default();
    // SetUpHeat :87-111, bottom branches.
    if k_bot_t > 0 {
        bottom.d = Some(1.0);
        bottom.e = Some(0.0);
        bottom.f = Some(t_bottom);
    } else if k_bot_t < 0 {
        let dx = x[1] - x[0];
        if level_two {
            // :92-97
            bottom.d = Some(
                dx / 2. / dt * cap[0]
                    + (cond[0] + cond[1]) / dx / 4.
                    + cw * (2. * v_new[0] + v_new[1]) / 12.
                    + dx / 24. * cw * (3. * sink[0] + sink[1]),
            );
            bottom.e = Some(
                -(cond[0] + cond[1]) / 4. / dx
                    + cw * (2. * v_new[1] + v_new[0]) / 12.
                    + dx / 24. * cw * (sink[0] + sink[1]),
            );
        } else {
            // :98-106
            bottom.f = Some(
                temp_old[0]
                    * (dx / 2. / dt * cap[0]
                        - (cond[0] + cond[1]) / dx / 4.
                        - cw * (2. * v_old[0] + v_old[1]) / 12.
                        - dx / 24. * cw * (3. * sink[0] + sink[1]))
                    + temp_old[1]
                        * ((cond[0] + cond[1]) / 4. / dx
                            - cw * (2. * v_old[1] + v_old[0]) / 12.
                            - dx / 24. * cw * (sink[0] + sink[1]))
                    + t_bottom * cw * (v_new[0] + v_old[0]) / 2.,
            );
        }
    } else {
        // :108-111, zero-gradient: -T1 + T2 = 0.
        bottom.d = Some(-1.0);
        bottom.e = Some(1.0);
        bottom.f = Some(0.0);
    }

    let last = n - 1;
    let prev = n - 2;
    let mut top = BoundaryCoefficients::default();
    // SetUpHeat :144-177, top branches.
    if k_top_t > 0 {
        top.b = Some(0.0);
        top.d = Some(1.0);
        top.f = Some(t_top_a);
    } else {
        let dx = x[last] - x[prev];
        if level_two {
            // :154-161
            top.b = Some(
                -(cond[last] + cond[prev]) / 4. / dx - cw * (v_new[last] + 2. * v_new[prev]) / 12.
                    + dx / 24. * cw * (sink[prev] + sink[last]),
            );
            top.d = Some(
                dx / 2. / dt * cap[last] + (cond[prev] + cond[last]) / 4. / dx
                    - cw * (2. * v_new[last] + v_new[prev]) / 12.
                    + dx / 24. * cw * (sink[prev] + 3. * sink[last]),
            );
        } else {
            // :163-175
            let mut f = temp_old[prev]
                * ((cond[last] + cond[prev]) / 4. / dx
                    + cw * (v_old[last] + 2. * v_old[prev]) / 12.
                    - dx / 24. * cw * (sink[prev] + sink[last]))
                + temp_old[last]
                    * (dx / 2. / dt * cap[last] - (cond[prev] + cond[last]) / 4. / dx
                        + cw * (2. * v_old[last] + v_old[prev]) / 12.
                        - dx / 24. * cw * (sink[prev] + 3. * sink[last]));
            // lEnBal is false in the Supported set, so both subtractions apply.
            f -= heat_flux;
            f -= heat_flux.min(0.0);
            top.f = Some(f);
        }
    }

    Ok(HeatBoundaryRows { bottom, top })
}

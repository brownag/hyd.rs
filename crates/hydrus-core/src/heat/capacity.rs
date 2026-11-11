//! Volumetric heat capacity and thermal conductivity, ported from `TempCap`
//! (`src_reference/TEMPER.FOR:269-323`).
//!
//! Material indices are zero-based here; the Fortran `MatNum` is one-based.
//! Outputs are written in place. After an error, the output slices are unspecified.

/// Heat-transport parameters for one material, in `TPar(1..9, M)` order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TempMaterial {
    /// `TPar(1)`: volume fraction of solids.
    pub solid_fraction: f64,
    /// `TPar(2)`: volume fraction of organic matter (inferred from usage).
    pub organic_fraction: f64,
    /// `TPar(3)`: dispersivity for the advective conductivity term (inferred from usage).
    pub dispersivity: f64,
    /// `TPar(4)`: volume fraction of quartz.
    pub quartz: f64,
    /// `TPar(5)`: volume fraction of other minerals.
    pub other_minerals: f64,
    /// `TPar(6)`: volume fraction of clay.
    pub clay: f64,
    /// `TPar(7)`: volumetric heat capacity of solids (inferred from usage).
    pub solid_heat_capacity: f64,
    /// `TPar(8)`: volumetric heat capacity of organic matter (inferred from usage).
    pub organic_heat_capacity: f64,
    /// `TPar(9)`: volumetric heat capacity of water.
    pub water_heat_capacity: f64,
}

impl TempMaterial {
    /// Builds a material from the nine `TPar` rows of one material column, in row order.
    pub fn from_tpar(rows: [f64; 9]) -> Self {
        Self {
            solid_fraction: rows[0],
            organic_fraction: rows[1],
            dispersivity: rows[2],
            quartz: rows[3],
            other_minerals: rows[4],
            clay: rows[5],
            solid_heat_capacity: rows[6],
            organic_heat_capacity: rows[7],
            water_heat_capacity: rows[8],
        }
    }

    fn named_values(&self) -> [(&'static str, f64); 9] {
        [
            ("solid_fraction", self.solid_fraction),
            ("organic_fraction", self.organic_fraction),
            ("dispersivity", self.dispersivity),
            ("quartz", self.quartz),
            ("other_minerals", self.other_minerals),
            ("clay", self.clay),
            ("solid_heat_capacity", self.solid_heat_capacity),
            ("organic_heat_capacity", self.organic_heat_capacity),
            ("water_heat_capacity", self.water_heat_capacity),
        ]
    }
}

/// Inputs to [`temp_cap`]. One entry per node in `material_indices`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TempCapInput<'a> {
    pub material_indices: &'a [usize],
    pub water_content: &'a [f64],
    pub velocity: &'a [f64],
    pub materials: &'a [TempMaterial],
    /// True selects the Campbell conductivity branch (`iCampbell == 1`).
    pub campbell: bool,
    pub x_conv: f64,
    pub t_conv: f64,
}

/// Typed rejections for [`temp_cap`]. Each variant is raised instead of a Fortran stop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TempCapError {
    LengthMismatch {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
    MaterialIndexOutOfRange {
        index: usize,
        material: usize,
        count: usize,
    },
    NonFiniteInput {
        name: &'static str,
        index: usize,
    },
    NonFiniteParameter {
        material: usize,
        name: &'static str,
    },
    InvalidConversion {
        name: &'static str,
        value: f64,
    },
    NegativeWaterContent {
        index: usize,
        value: f64,
    },
    NonPositiveCapacity {
        index: usize,
        value: f64,
    },
}

/// Computes volumetric heat capacity `cap` and thermal conductivity `cond` per node.
///
/// Follows `TempCap` in source order: `Cap = TPar(7)*TPar(1) + TPar(8)*TPar(2) + TPar(9)*th`,
/// then the default or Campbell conductivity, then the advective term `TPar(9)*TPar(3)*|v|`.
pub fn temp_cap(
    input: TempCapInput<'_>,
    cap: &mut [f64],
    cond: &mut [f64],
) -> Result<(), TempCapError> {
    let TempCapInput {
        material_indices,
        water_content,
        velocity,
        materials,
        campbell,
        x_conv,
        t_conv,
    } = input;
    let count = material_indices.len();
    for (name, actual) in [
        ("water_content", water_content.len()),
        ("velocity", velocity.len()),
        ("cap", cap.len()),
        ("cond", cond.len()),
    ] {
        if actual != count {
            return Err(TempCapError::LengthMismatch {
                name,
                expected: count,
                actual,
            });
        }
    }
    for (material, parameters) in materials.iter().enumerate() {
        for (name, value) in parameters.named_values() {
            if !value.is_finite() {
                return Err(TempCapError::NonFiniteParameter { material, name });
            }
        }
    }
    if campbell {
        for (name, value) in [("x_conv", x_conv), ("t_conv", t_conv)] {
            if !value.is_finite() || value <= 0.0 {
                return Err(TempCapError::InvalidConversion { name, value });
            }
        }
    }
    for (index, ((&material_index, &theta), &v)) in material_indices
        .iter()
        .zip(water_content)
        .zip(velocity)
        .enumerate()
    {
        if material_index >= materials.len() {
            return Err(TempCapError::MaterialIndexOutOfRange {
                index,
                material: material_index,
                count: materials.len(),
            });
        }
        if !theta.is_finite() {
            return Err(TempCapError::NonFiniteInput {
                name: "water_content",
                index,
            });
        }
        if !v.is_finite() {
            return Err(TempCapError::NonFiniteInput {
                name: "velocity",
                index,
            });
        }
        if theta < 0.0 {
            return Err(TempCapError::NegativeWaterContent {
                index,
                value: theta,
            });
        }
        let m = &materials[material_index];
        let capacity = m.solid_heat_capacity * m.solid_fraction
            + m.organic_heat_capacity * m.organic_fraction
            + m.water_heat_capacity * theta;
        if capacity <= 0.0 {
            return Err(TempCapError::NonPositiveCapacity {
                index,
                value: capacity,
            });
        }
        let base = if campbell {
            campbell_conductivity(m, theta, x_conv, t_conv)
        } else {
            (m.quartz + m.other_minerals * theta + m.clay * theta.sqrt()).max(0.0)
        };
        cap[index] = capacity;
        cond[index] = base + m.water_heat_capacity * m.dispersivity * v.abs();
    }
    Ok(())
}

/// Campbell conductivity branch of `TempCap` (`TEMPER.FOR:287-302`), before the advective term.
fn campbell_conductivity(m: &TempMaterial, theta: f64, x_conv: f64, t_conv: f64) -> f64 {
    let s = m.solid_fraction;
    let q = m.quartz;
    let o = m.other_minerals;
    let aa = (0.57 + 1.73 * q + 0.93 * o) / (1.0 - 0.74 * q - 0.49 * o) - 2.8 * s * (1.0 - s);
    let bb = 2.8 * s;
    let xc = m.clay.max(0.005);
    let cc = 1.0 + 2.6 / xc.sqrt();
    let dd = 0.03 + 0.7 * (s * s);
    // Fortran `-(CC*th)**EE` with EE = 4.: the minus applies to the power.
    let xlamb = aa + bb * theta - (aa - dd) * (-(cc * theta).powf(4.0)).exp();
    (xlamb * x_conv / t_conv / t_conv / t_conv).max(0.0)
}

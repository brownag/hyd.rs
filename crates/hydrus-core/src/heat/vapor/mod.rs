//! Closed first-increment vapor selection and downstream data contracts.
//!
//! Solver implementations are owned by later tasks; these modules intentionally
//! contain documentation-only placeholders until their assigned wave.

pub mod adjust;
pub mod balance;
pub mod driver;
pub mod kernels;
pub mod output;
pub mod state;
pub mod water;

use std::fmt;

use crate::heat::config::{HeatConfig, HeatConfigError, HeatConfigInput};
use crate::hydraulics::{HydraulicTables, VanGenuchtenModel};
use crate::{
    HydraulicError, LowerBoundaryMode, UpperBoundary, WaterState, WaterStepInput,
    WaterStepPolicy, WaterWorkspace,
};

/// Raw inputs for the closed supported vapor selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaporConfigInput {
    pub heat: HeatConfigInput,
    pub water_enabled: bool,
    pub heat_enabled: bool,
    pub i_enhanc: i32,
    pub hysteresis: i32,
    pub chemistry: bool,
    pub density_coupling: bool,
    pub centrifugal: bool,
    pub top_temperature: f64,
    pub bottom_temperature: f64,
    pub x_conv: f64,
    pub t_conv: f64,
}

/// Validated, closed vapor configuration for the first supported selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaporConfig {
    heat: HeatConfig,
    top_temperature: f64,
    bottom_temperature: f64,
    x_conv: f64,
    t_conv: f64,
}

impl VaporConfig {
    pub fn new(input: VaporConfigInput) -> Result<Self, VaporError> {
        let heat = HeatConfig::new(input.heat).map_err(VaporError::Heat)?;
        if !input.water_enabled {
            return Err(VaporError::UnsupportedBoolean {
                name: "water_enabled",
                value: false,
            });
        }
        if !input.heat_enabled {
            return Err(VaporError::UnsupportedBoolean {
                name: "heat_enabled",
                value: false,
            });
        }
        if !heat.vapor_enabled() {
            return Err(VaporError::UnsupportedBoolean {
                name: "l_vapor",
                value: false,
            });
        }
        if input.i_enhanc != 1 {
            return Err(VaporError::UnsupportedInteger {
                name: "iEnhanc",
                value: input.i_enhanc,
            });
        }
        if input.hysteresis != 0 {
            return Err(VaporError::UnsupportedInteger {
                name: "hysteresis",
                value: input.hysteresis,
            });
        }
        for (name, value) in [
            ("chemistry", input.chemistry),
            ("density_coupling", input.density_coupling),
            ("centrifugal", input.centrifugal),
        ] {
            if value {
                return Err(VaporError::UnsupportedBoolean { name, value });
            }
        }
        for (name, value) in [
            ("top_temperature", input.top_temperature),
            ("bottom_temperature", input.bottom_temperature),
        ] {
            validate_finite(name, value)?;
            if value + 273.15 <= 0.0 {
                return Err(VaporError::InvalidValue { name, value });
            }
        }
        for (name, value) in [("xConv", input.x_conv), ("tConv", input.t_conv)] {
            validate_finite(name, value)?;
            if value <= 0.0 {
                return Err(VaporError::InvalidValue { name, value });
            }
        }
        Ok(Self {
            heat,
            top_temperature: input.top_temperature,
            bottom_temperature: input.bottom_temperature,
            x_conv: input.x_conv,
            t_conv: input.t_conv,
        })
    }

    pub fn heat(&self) -> HeatConfig { self.heat }
    pub fn top_temperature(&self) -> f64 { self.top_temperature }
    pub fn bottom_temperature(&self) -> f64 { self.bottom_temperature }
    pub fn x_conv(&self) -> f64 { self.x_conv }
    pub fn t_conv(&self) -> f64 { self.t_conv }
    pub fn enhancement(&self) -> i32 { 1 }
    pub fn hydraulic_model(&self) -> i32 { 0 }

    /// Validates all caller water inputs without mutating state or workspace.
    pub fn validate_water_input(
        &self,
        input: &WaterStepInput<'_>,
        accepted_time: f64,
        policy: WaterStepPolicy,
    ) -> Result<(), VaporError> {
        let mesh = input.mesh;
        let n = mesh.coordinates().len();
        if n < 3 {
            return Err(VaporError::TooFewNodes { count: n });
        }
        if !accepted_time.is_finite() || !input.target_time.is_finite()
            || input.target_time <= accepted_time
        {
            return Err(VaporError::InvalidTime);
        }
        if !input.timestep.is_finite() || input.timestep <= 0.0
            || input.timestep < policy.minimum_timestep
        {
            return Err(VaporError::InvalidTime);
        }
        if policy.max_iterations == 0 || !policy.head_tolerance.is_finite()
            || policy.head_tolerance <= 0.0 || !policy.water_tolerance.is_finite()
            || policy.water_tolerance <= 0.0 || !policy.minimum_timestep.is_finite()
            || policy.minimum_timestep <= 0.0 || !policy.retry_factor.is_finite()
            || policy.retry_factor <= 1.0 || !policy.minimum_pivot.is_finite()
            || policy.minimum_pivot <= 0.0
        {
            return Err(VaporError::InvalidPolicy);
        }
        if input.face_averaging != crate::hydraulics::FaceAveraging::Arithmetic {
            return Err(VaporError::UnsupportedAveraging);
        }
        if input.root_uptake.is_some() {
            return Err(VaporError::RootUptake);
        }
        for (index, &sink) in input.sink.iter().enumerate() {
            validate_finite_index("sink", index, sink)?;
            if sink != 0.0 {
                return Err(VaporError::NonzeroSink { index, value: sink });
            }
        }
        check_len("sink", n, input.sink.len())?;
        check_len("material_indices", n, mesh.material_indices().len())?;
        if mesh.material_count() != input.materials.len() {
            return Err(VaporError::LengthMismatch {
                name: "mesh material table",
                expected: mesh.material_count(),
                actual: input.materials.len(),
            });
        }
        for (i, &material) in mesh.material_indices().iter().enumerate() {
            if material >= input.materials.len() {
                return Err(VaporError::MaterialIndex { index: i, material });
            }
        }
        for (i, &x) in mesh.coordinates().iter().enumerate() {
            validate_finite_index("coordinates", i, x)?;
            if i > 0 && x <= mesh.coordinates()[i - 1] {
                return Err(VaporError::InvalidValue { name: "coordinates", value: x });
            }
        }
        match input.lower {
            LowerBoundaryMode::FixedHead { head, row, flux } => {
                validate_finite("lower head", head)?;
                validate_finite("lower flux", flux)?;
                validate_row("lower row", row.lower, row.diagonal, row.upper, row.rhs)?;
            }
            mode => return Err(VaporError::UnsupportedLowerBoundary(mode)),
        }
        match input.upper {
            UpperBoundary::Head { head, row, flux } => {
                validate_finite("upper head", head)?;
                validate_finite("upper flux", flux)?;
                validate_row("upper row", row.lower, row.diagonal, row.upper, row.rhs)?;
            }
            mode => return Err(VaporError::UnsupportedUpperBoundary(mode)),
        }
        Ok(())
    }
}

/// Typed rejection from vapor selection, preflight, or primitive validation.
#[derive(Clone, Debug, PartialEq)]
pub enum VaporError {
    Heat(HeatConfigError),
    UnsupportedBoolean { name: &'static str, value: bool },
    UnsupportedInteger { name: &'static str, value: i32 },
    UnsupportedLowerBoundary(LowerBoundaryMode),
    UnsupportedUpperBoundary(UpperBoundary),
    UnsupportedAveraging,
    RootUptake,
    NonzeroSink { index: usize, value: f64 },
    TooFewNodes { count: usize },
    LengthMismatch { name: &'static str, expected: usize, actual: usize },
    MaterialIndex { index: usize, material: usize },
    NonFinite { name: &'static str, index: Option<usize>, value: f64 },
    InvalidValue { name: &'static str, value: f64 },
    InvalidTime,
    InvalidPolicy,
    Hydraulic(HydraulicError),
}

impl fmt::Display for VaporError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "vapor contract error: {self:?}") }
}
impl std::error::Error for VaporError {}

/// Explicit primitive arrays required by the material kernels.
#[derive(Clone, Copy, Debug)]
pub struct VaporPrimitiveInput<'a> {
    pub heads: &'a [f64],
    pub temperatures: &'a [f64],
    pub liquid_water_contents: &'a [f64],
    pub liquid_conductivities: &'a [f64],
    pub liquid_capacities: &'a [f64],
    pub material_indices: &'a [usize],
    pub saturated_water_contents: &'a [f64],
    pub x_conv: f64,
    pub t_conv: f64,
}

impl VaporPrimitiveInput<'_> {
    pub fn validate(&self) -> Result<(), VaporError> {
        let n = self.heads.len();
        if n < 3 { return Err(VaporError::TooFewNodes { count: n }); }
        for (name, len) in [("temperatures", self.temperatures.len()), ("liquid_water_contents", self.liquid_water_contents.len()), ("liquid_conductivities", self.liquid_conductivities.len()), ("liquid_capacities", self.liquid_capacities.len()), ("material_indices", self.material_indices.len())] { check_len(name, n, len)?; }
        for (name, values) in [("heads", self.heads), ("temperatures", self.temperatures), ("liquid_water_contents", self.liquid_water_contents), ("liquid_conductivities", self.liquid_conductivities), ("liquid_capacities", self.liquid_capacities), ("saturated_water_contents", self.saturated_water_contents)] {
            for (i, &v) in values.iter().enumerate() { validate_finite_index(name, i, v)?; }
        }
        for (name, value) in [("xConv", self.x_conv), ("tConv", self.t_conv)] { validate_finite(name, value)?; if value <= 0.0 { return Err(VaporError::InvalidValue { name, value }); } }
        for (i, (&theta, &material)) in self.liquid_water_contents.iter().zip(self.material_indices).enumerate() {
            let theta_s = *self.saturated_water_contents.get(material).ok_or(VaporError::MaterialIndex { index: i, material })?;
            if theta < 0.0 || theta > theta_s || theta_s <= 0.0 { return Err(VaporError::InvalidValue { name: "water_content/porosity", value: theta }); }
            if self.liquid_conductivities[i] < 0.0 { return Err(VaporError::InvalidValue { name: "liquid_conductivity", value: self.liquid_conductivities[i] }); }
            if self.liquid_capacities[i] < 0.0 { return Err(VaporError::InvalidValue { name: "liquid_capacity", value: self.liquid_capacities[i] }); }
            if self.temperatures[i] + 273.15 <= 0.0 { return Err(VaporError::InvalidValue { name: "absolute_temperature", value: self.temperatures[i] }); }
        }
        Ok(())
    }
}

/// Owned properties returned by vapor material evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct VaporProperties {
    pub liquid_conductivities: Vec<f64>,
    pub liquid_thermal_conductivities: Vec<f64>,
    pub vapor_head_conductivities: Vec<f64>,
    pub vapor_thermal_conductivities: Vec<f64>,
    pub vapor_contents: Vec<f64>,
    pub effective_hydraulic_capacities: Vec<f64>,
}

/// Explicit concrete model/table choice for later source-matched evaluation.
#[derive(Clone, Copy)]
pub struct VaporSourceMaterial<'a> { pub model: &'a VanGenuchtenModel, pub table: Option<&'a HydraulicTables> }

/// Immutable accepted physical history and separately retained predictor.
#[derive(Clone, Copy, Debug)]
pub struct VaporHistoryView<'a> {
    pub time: f64,
    pub previous_timestep: Option<f64>,
    pub heads: &'a [f64],
    pub liquid_water_contents: &'a [f64],
    pub temperatures: &'a [f64],
    pub vapor_contents: &'a [f64],
    pub liquid_velocities: &'a [f64],
    pub vapor_velocities: &'a [f64],
    pub head_predictor: &'a [f64],
}

impl VaporHistoryView<'_> {
    pub fn validate(&self) -> Result<(), VaporError> {
        validate_finite("history time", self.time)?;
        if let Some(dt) = self.previous_timestep { validate_finite("previous timestep", dt)?; if dt <= 0.0 { return Err(VaporError::InvalidValue { name: "previous timestep", value: dt }); } }
        let n = self.heads.len(); if n < 3 { return Err(VaporError::TooFewNodes { count: n }); }
        for (name, values) in [("liquid_water_contents", self.liquid_water_contents), ("temperatures", self.temperatures), ("vapor_contents", self.vapor_contents), ("liquid_velocities", self.liquid_velocities), ("vapor_velocities", self.vapor_velocities), ("head_predictor", self.head_predictor)] { check_len(name, n, values.len())?; for (i, &v) in values.iter().enumerate() { validate_finite_index(name, i, v)?; } }
        for (i, &t) in self.temperatures.iter().enumerate() { if t + 273.15 <= 0.0 { return Err(VaporError::InvalidValue { name: "absolute_temperature", value: t }); } if self.vapor_contents[i] < 0.0 { return Err(VaporError::InvalidValue { name: "vapor_contents", value: self.vapor_contents[i] }); } }
        for (i, &h) in self.heads.iter().enumerate() { validate_finite_index("heads", i, h)?; }
        Ok(())
    }
}

/// Inputs shared by the two source-ordered water passes.
#[derive(Clone, Copy)]
pub struct VaporWaterContext<'a> {
    pub config: &'a VaporConfig,
    pub old: VaporHistoryView<'a>,
    pub temperature_estimate: &'a [f64],
    pub initial_heads: &'a [f64],
    pub initial_material_heads: &'a [f64],
    pub source_materials: &'a [VaporSourceMaterial<'a>],
}

/// Owned uncommitted water trial and phase diagnostics.
#[derive(Clone, Debug)]
pub struct VaporWaterTrial {
    pub candidate: WaterState,
    pub workspace: WaterWorkspace,
    pub accepted_time: f64,
    pub timestep: f64,
    pub iterations: usize,
    pub retries: usize,
    pub pre_solve_heads: Vec<f64>,
    pub pre_solve_liquid_water_contents: Vec<f64>,
    pub pre_solve_liquid_capacities: Vec<f64>,
    pub liquid_velocities: Vec<f64>,
    pub vapor_velocities: Vec<f64>,
    pub properties: VaporProperties,
    pub liquid_face_fluxes: Vec<f64>,
    pub vapor_face_fluxes: Vec<f64>,
    pub total_face_fluxes: Vec<f64>,
}

/// Vapor additions to an otherwise unchanged liquid-only heat input.
#[derive(Clone, Copy, Debug)]
pub struct VaporHeatInput<'a> {
    pub old: VaporHistoryView<'a>,
    pub temperature_estimate: &'a [f64],
    pub vapor_contents_new: &'a [f64],
    pub vapor_velocities_new: &'a [f64],
}

fn validate_finite(name: &'static str, value: f64) -> Result<(), VaporError> {
    if !value.is_finite() { Err(VaporError::NonFinite { name, index: None, value }) } else { Ok(()) }
}
fn validate_finite_index(name: &'static str, index: usize, value: f64) -> Result<(), VaporError> {
    if !value.is_finite() { Err(VaporError::NonFinite { name, index: Some(index), value }) } else { Ok(()) }
}
fn check_len(name: &'static str, expected: usize, actual: usize) -> Result<(), VaporError> {
    if expected != actual { Err(VaporError::LengthMismatch { name, expected, actual }) } else { Ok(()) }
}
fn validate_row(name: &'static str, lower: f64, diagonal: f64, upper: f64, rhs: f64) -> Result<(), VaporError> {
    for value in [lower, diagonal, upper, rhs] { validate_finite(name, value)?; }
    Ok(())
}

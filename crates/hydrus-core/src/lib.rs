//! Filesystem-free numerical contracts for HYDRUS hydraulic models.

pub mod error;
pub mod hydraulics;
mod water_solver;
mod water_state;

pub use error::{HydraulicError, WaterStepError};
pub use hydraulics::{
    Evaluation, HydraulicDerivatives, HydraulicModel, HydraulicModelSelector, HydraulicTables,
    Mesh1D, VanGenuchtenModel, VanGenuchtenParameters,
};
pub use water_solver::{
    advance_water_step, WaterBalance, WaterStepInput, WaterStepPolicy, WaterStepReport,
};
pub use water_state::{WaterAccounting, WaterState, WaterWorkspace};

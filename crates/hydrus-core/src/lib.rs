//! Filesystem-free numerical contracts for HYDRUS hydraulic models.

pub mod atmospheric;
pub mod error;
pub mod forcing;
pub mod hydraulics;
mod water_solver;
mod water_state;

pub use atmospheric::{advance_atmospheric_step, AtmosphericState, UpperBoundaryMode};
pub use error::{HydraulicError, WaterStepError};
pub use forcing::{AtmosphericRates, ForcingEvent, ForcingSchedule, ForcingUnits};
pub use hydraulics::{
    Evaluation, HydraulicDerivatives, HydraulicModel, HydraulicModelSelector, HydraulicTables,
    Mesh1D, UnitSymbol, VanGenuchtenModel, VanGenuchtenParameters,
};
pub use water_solver::{
    advance_water_step, WaterBalance, WaterProductConfig, WaterStepInput, WaterStepPolicy,
    WaterStepReport,
};
pub use water_state::{WaterAccounting, WaterState, WaterWorkspace};

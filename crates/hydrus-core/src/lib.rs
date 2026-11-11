//! Filesystem-free numerical contracts for HYDRUS hydraulic models.
//!
//! Solute transport defaults to [`SoluteBackend::FortranCompatible`] based on all-mode
//! parity verification against the pinned Fortran reference (see `planning/master/SOLUTE_FORTRAN_PARITY_18.md`).
//! The [`SoluteBackend::FiniteVolumeBackwardEuler`] backend remains explicitly selectable.

pub mod atmospheric;
pub mod error;
pub mod forcing;
pub mod heat;
pub mod hydraulics;
pub mod lower_boundary;
pub mod root_uptake;
pub mod solute;
mod solute_fortran_compat;
mod water_solver;
mod water_state;

pub use atmospheric::{advance_atmospheric_step, AtmosphericState, UpperBoundaryMode};
pub use error::{HydraulicError, SoluteConfigError, WaterStepError};
pub use forcing::{AtmosphericRates, ForcingEvent, ForcingSchedule, ForcingUnits};
pub use hydraulics::{
    Evaluation, HydraulicDerivatives, HydraulicModel, HydraulicModelSelector, HydraulicTables,
    Mesh1D, UnitSymbol, VanGenuchtenModel, VanGenuchtenParameters,
};
pub use lower_boundary::{LowerBoundaryMode, UnsupportedLowerBoundaryMode};
pub use root_uptake::{feddes_reduction, RootUptakeInput, RootUptakeModel};
pub use solute::{
    advance_solute_step, DeferredSoluteFeature, EquilibriumSorption, KineticSorption,
    SoluteAccounting, SoluteBackend, SoluteBoundaries, SoluteBoundaryMode, SoluteConfig,
    SoluteEndpoint, SoluteSpecies, SoluteState, SoluteStepError, SoluteTransportInput, SoluteUnits,
    SoluteWorkspace,
};
pub use solute_fortran_compat::compat_runner::{run_compat_deck, CompatRunOutputs};
pub use water_solver::{
    advance_water_step, UpperBoundary, WaterBalance, WaterProductConfig, WaterStepInput,
    WaterStepPolicy, WaterStepReport,
};
pub use water_state::{AcceptedWaterSnapshot, WaterAccounting, WaterState, WaterWorkspace};

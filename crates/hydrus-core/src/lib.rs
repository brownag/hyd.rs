//! Filesystem-free numerical contracts for HYDRUS hydraulic models.

pub mod error;
pub mod hydraulics;

pub use error::HydraulicError;
pub use hydraulics::{
    Evaluation, HydraulicDerivatives, HydraulicModel, HydraulicModelSelector, HydraulicTables,
    Mesh1D, VanGenuchtenModel, VanGenuchtenParameters,
};
